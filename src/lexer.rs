use std::os::unix::io::RawFd;
use std::str::{self,CharIndices};

pub type Span<T, E> = Result<(usize, T, usize), E>;

#[derive(Debug)]
pub enum Error {
    UnrecognizedChar(usize, char, usize),
}

pub enum Token<'input> {
    Space,
    Tab,
    Newline,
    Linefeed,
    Semicolon,
    DoubleSemicolon,
    Pipe,
    Ampersand,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Backtick,
    Bang,
    Dollar,
    DoubleQuote,
    SingleQuote,
    Equals,
    Backslash,
    Asterisk,
    QuestionMark,
    Comment,
    GreaterThan,
    LessThan,
    DoubleGreaterThan,
    DoubleLessThan,
    LessThanAnd,
    GreaterThanAnd,
    And,
    Or,
    DoubleSemicolon,
    If,
    Then,
    Else,
    Elif,
    Fi,
    Do,
    Done,
    Case,
    Esac,
    While,
    Until,
    For,
    In,
    Word(&'input str),
    IoNumber(RawFd),
    Text(&'input str),
}

pub struct Lexer<'input> {
    input: &'input str,
    chars: CharIndices<'input>,
    lookahead: Option<(usize, char, usize)>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Lexer<'input> {
        let mut chars = input.char_indices();
        let next = chars.next();
        let lookahead = next.map(|n| (n.0, n.1, n.0 + n.1.len_utf8()));
        Lexer {
            input,
            chars,
            lookahead,
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Span<Token<'input>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        
        while let Some((start, chr, end)) = self.advance() {
            let token = match chr {
                '\n' => Some(Ok((start, Token::Newline, end))),
                ';' => {
                    match self.lookahead {
                        Some((_, ';', _)) => {
                            self.advance();
                            Some(Ok((start, Token::DoubleSemicolon, end)))
                        },
                        _ => Some(Ok((start, Token::Semicolon, end))),
                    }
                },
                '#' => {
                    while let Some((_, chr, _)) = self.lookahead {
                        match chr {
                            '\n' => break,
                            _ => self.advance(),
                        };
                    }
                    self.next()
                }
                ')' => Some(Ok((start, Token::RightParen, end))),
                '(' => Some(Ok((start, Token::LeftParen, end))),
                '`' => Some(Ok((start, Token::Backtick, end))),
                '!' => Some(Ok((start, Token::Bang, end))),
                '=' => Some(Ok((start, Token::Equals, end))),
                '\\' => Some(Ok((start, Token::Backslash, end))),
                '\'' => Some(self.single_quote(start, end)),
                '"' => Some(self.double_quote(start, end)),
                '>' => {
                    match self.lookahead {
                        Some((_, '>', _)) => {
                            self.advance();
                            Some(Ok((start, Token::DoubleGreaterThan, end)))
                        },
                        Some((_, '&', _)) => {
                            self.advance();
                            Some(Ok((start, Token::GreaterThanAnd, end)))
                        },
                        _ => Some(Ok((start, Token::GreaterThan, end))),
                    }
                },
                '<' => {
                    match self.lookahead {
                        Some((_, '<', _)) => {
                            self.advance();
                            Some(Ok((start, Token::DoubleLessThan, end)))
                        },
                        Some((_, '&', _)) => {
                            self.advance();
                            Some(Ok((start, Token::LessThanAnd, end)))
                        },
                        _ => Some(Ok((start, Token::LessThan, end))),
                    }
                },
                '&' => {
                    match self.lookahead {
                        Some((_, '&', _)) => {
                            self.advance();
                            Some(Ok((start, Token::And, end)))
                        },
                        _ => Some(Ok((start, Token::Ampersand, end))),
                    }
                },
                '|' => {
                    match self.lookahead {
                        Some((_, '|', _)) => {
                            self.advance();
                            Some(Ok((start, Token::Or, end)))
                        },
                        _ => Some(Ok((start, Token::Pipe, end))),
                    }
                },
                '$' => {
                    match self.lookahead {
                        Some((_, '(', _)) | Some((_, '{', _)) => {
                            Some(Ok((start, Token::Dollar, end)))
                        }
                         _ => Some(self.word(start,end)),
                        
                    }
                },
                '{' => Some(self.block(start, start + end)),
                '}' => Some(Ok((start, Token::RightBrace, end))),
                chr if is_word_start(chr) => Some(self.word(start, end)),
                chr if chr.is_whitespace() => continue,
                chr => Some(Err(Error::UnrecognizedChar(start, chr, end))),
            };

            return token;
        }

        let token = None;

        token
    }
}

impl<'input> Lexer<'input> {
    fn advance(&mut self) -> Option<(usize, char, usize)> {
        match self.lookahead {
            Some((start, chr, end)) => {
                let next = self.chars.next();
                self.lookahead = next.map(|n| (n.0, n.1, n.0 + n.1.len_utf8()));
                Some((start, chr, end))
            },
            None => None,
        }
    }

    fn take_until<F>(&mut self, start: usize, mut end: usize, mut terminate: F) -> (&'input str, usize)
        where F: FnMut(char) -> bool{
        while let Some((_, chr, _)) = self.lookahead {
            if terminate(chr) {
                return (&self.input[start..end], end);
            }
            else if let Some((_, _, e)) = self.advance() {
                end = e;
            }
        }

        (&self.input[start..end], end)
    }

    fn take_while<F>(&mut self, start: usize, end: usize, mut keep_going: F) -> (&'input str, usize)
        where F: FnMut(char) -> bool {
        self.take_until(start, end, |chr| !keep_going(chr))
    }
    
    fn single_quote(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        let (_, end) = self.take_while(start, end, |chr| chr != '\'');
        self.advance();
        Ok((start, Token::Word(&self.input[start+1..end]), end))
    }

    fn double_quote(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        let (_, end) = self.take_while(start, end, |chr| chr != '"');
        self.advance();
        Ok((start, Token::Word(&self.input[1..]), end))
    }

    fn word(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        let (word, end) = self.take_while(start, end, is_word_continue);
        let tok = match word {
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "elif" => Token::Elif,
            "fi" => Token::Fi,
            "for" => Token::For,
            "in" => Token::In,
            "do" => Token::Do,
            "done" => Token::Done,
            "while" => Token::While,
            "until" => Token::Until,
            "case" => Token::Case,
            "esac" => Token::Esac,
            word => self.io_number(word),
        };

        Ok((start, tok, end))
    }

    fn io_number<'a>(&mut self, word: &'a str) -> Token<'a> {
        if let Some((_, chr, _)) = self.lookahead {
            if chr == '>' || chr == '<' {
                if let Ok(num) = word.parse::<i32>() {
                    return Token::IoNumber(num);
                }
            }
        }

        Token::Word(word)
    }

    fn block(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {


        Ok((start, Token::LeftBrace, end))
    }

    fn text(&mut self, start: usize, end: usize ) -> Result<(usize, Token<'input>, usize), Error> {
        let (_, end) = self.take_until(start, end, |chr| chr == '}');

        Ok((start, Token::Text(&self.input[start..end]), end))
    }
}

fn is_word_start(chr: char) -> bool {
    match chr {
        '\u{007F}' |
        '\u{0000}'..='\u{001F}' |
        '\u{0080}'..='\u{009F}' => false,
        _ => is_word_continue(chr),
    }
}

fn is_word_continue(chr: char) -> bool {
    match chr {
        ';' | ')' | '(' | '`' | '!' | '=' | '\\' | '\'' | '"'
            | '>' | '<' | '&' | '|' | '{' | '}' | '*'
          => false,
        _ => !chr.is_whitespace()
    }
}


