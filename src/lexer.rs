use std::os::unix::io::RawFd;
use std::str::{self, CharIndices};
use std::fmt::{self, Display, Formatter};

/// A result type wrapping a token with start and end locations.
pub type Span<T, E> = Result<(usize, T, usize), E>;

/// A lexer error.
#[derive(Debug)]
pub enum Error {
    UnrecognizedChar(usize, char, usize),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::UnrecognizedChar(start, chr, end) => {
                write!(f, "Unrecognized character '{}' at position {}", chr, start)
            }
        }
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum Token<'input> {
    Space,
    Tab,
    Newline,
    NewlineList,
    Comment,
    SemiColon,
    Pipe,
    BackTick,
    Dollar,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Greater,
    Less,
    DoubleGreater,
    DoubleLess,
    GreaterAnd,
    LessAnd,
    Ampersand,
    Equals,
    And,
    Or,
    Bang,
    For,
    In,
    While,
    Until,
    If,
    Then,
    Else,
    Elif,
    Fi,
    Do,
    Done,
    Case,
    Esac,
/*    Break,
    Continue,
    Return,*/
    EOF,
    Subshell(&'input str),
    Number(RawFd),
    Word(&'input str),
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Token::Space => write!(f, "Space"),
            Token::Tab => write!(f, "Tab"),
            Token::Newline => write!(f, "Newline"),
            Token::NewlineList => write!(f, "NewlineList"),
            Token::Comment => write!(f, "Comment"),
            Token::SemiColon => write!(f, "SemiColon"),
            Token::Pipe => write!(f, "Pipe"),
            Token::BackTick => write!(f, "BackTick"),
            Token::Dollar => write!(f, "Dollar"),
            Token::OpenParen => write!(f, "OpenParen"),
            Token::CloseParen => write!(f, "CloseParen"),
            Token::OpenBrace => write!(f, "OpenBrace"),
            Token::CloseBrace => write!(f, "CloseBrace"),
            Token::Greater => write!(f, "Greater"),
            Token::Less => write!(f, "Less"),
            Token::DoubleGreater => write!(f, "DoubleGreater"),
            Token::DoubleLess => write!(f, "DoubleLess"),
            Token::GreaterAnd => write!(f, "GreaterAnd"),
            Token::LessAnd => write!(f, "LessAnd"),
            Token::Ampersand => write!(f, "Ampersand"),
            Token::Equals => write!(f, "Equals"),
            Token::And => write!(f, "And"),
            Token::Or => write!(f, "Or"),
            Token::Bang => write!(f, "Bang"),
            Token::For => write!(f, "For"),
            Token::In => write!(f, "In"),
            Token::While => write!(f, "While"),
            Token::Until => write!(f, "Until"),
            Token::If => write!(f, "If"),
            Token::Then => write!(f, "Then"),
            Token::Else => write!(f, "Else"),
            Token::Elif => write!(f, "Elif"),
            Token::Fi => write!(f, "Fi"),
            Token::Do => write!(f, "Do"),
            Token::Done => write!(f, "Done"),
            Token::Case => write!(f, "Case"),
            Token::Esac => write!(f, "Esac"),
            /*Token::Break => write!(f, "Break"),
            Token::Continue => write!(f, "Continue"),
            Token::Return => write!(f, "Return"),*/
            Token::EOF => write!(f, "EOF"),
            Token::Subshell(s) => write!(f, "Subshell({})", s),
            Token::Number(n) => write!(f, "Number({})", n),
            Token::Word(s) => write!(f, "Word({})", s),
        }
    }
}


pub struct Lexer<'input> {
    send_eof: bool,
    input: &'input str,
    chars: CharIndices<'input>,
    lookahead: Option<(usize, char, usize)>,
                    //size, char, end postion
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Lexer<'input> {
        let mut chars = input.char_indices();
        let next = chars.next();
        let lookahead = next.map(|n| (n.0, n.1, n.0 + n.1.len_utf8()));
        Lexer {
            send_eof: false,
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
            //eprintln!("{}: {}", start, chr);
            let token = match chr {
                '\n' => Some(self.newline_list(start, end)),
                ';' => Some(Ok((start, Token::SemiColon, end))),
                '|' => {
                    match self.lookahead {
                        Some((_, '|', _)) => {
                            self.advance();
                            Some(Ok((start, Token::Or, end)))
                        },
                        _ => Some(Ok((start, Token::Pipe, end))),
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
                '`' => Some(self.subshell(chr,start, end)),
                '$' => {
                    match self.lookahead {
                        Some((_, '(', _)) => {
                            Some(self.subshell(chr,start, end))
                        }

                        Some((_, '{', _)) => {

                            Some(Ok((start, Token::Dollar, end)))
                        }
                         _ => Some(self.word(start,end)),
                        
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
                },
                '(' => Some(Ok((start, Token::OpenParen, end))),
                ')' => Some(Ok((start, Token::CloseParen, end))),
                '{' => Some(Ok((start, Token::OpenBrace, end))),
                '}' => Some(Ok((start, Token::CloseBrace, end))),
                '>' => {
                    match self.lookahead {
                        Some((_, '>', _)) => {
                            self.advance();
                            Some(Ok((start, Token::DoubleGreater, end)))
                        },
                        Some((_, '&', _)) => {
                            self.advance();
                            Some(Ok((start, Token::GreaterAnd, end)))
                        },
                        _ => Some(Ok((start, Token::Greater, end))),
                    }
                },
                '<' => {
                    match self.lookahead {
                        Some((_, '<', _)) => {
                            self.advance();
                            Some(Ok((start, Token::DoubleLess, end)))
                        },
                        Some((_, '&', _)) => {
                            self.advance();
                            Some(Ok((start, Token::LessAnd, end)))
                        },
                        _ => Some(Ok((start, Token::Less, end))),
                    }
                },
                '=' => Some(Ok((start, Token::Equals, end))),
                '"' => Some(self.double_quote(start, end)),
                '\'' => Some(self.single_quote(start, end)),
                '!' => Some(Ok((start, Token::Bang, end))),
                chr if is_word_start(chr) => Some(self.word(start, end)),
                chr if chr.is_whitespace() => continue,
                chr => Some(Err(Error::UnrecognizedChar(start, chr, end))),
            };
            return token;
        }

        if !self.send_eof {
            self.send_eof = true;
            return Some(Ok((0, Token::EOF, 0)));
        }
        

        return None;
            

    }
}

impl<'input> Lexer<'input> {
    fn advance(&mut self) -> Option<(usize, char, usize)> {
        match self.lookahead {
            Some((start, chr, end)) => {
                let next = self.chars.next();
                self.lookahead = next.map(|n| (n.0, n.1, n.0 + n.1.len_utf8()));
                Some((start, chr, end))
            }
            None => None,
        }
    }

    fn take_until<F>(&mut self, start: usize, mut end: usize,  mut terminate: F)
        -> (&'input str, usize)
        where F: FnMut(char) -> bool
    {
        while let Some((_, c, _)) = self.lookahead {
            if terminate(c) {
                return (&self.input[start..end], end);
            } else if let Some((_, _, e)) = self.advance() {
                end = e;
            }
        } 
        (&self.input[start..end], end)
    }
    
    fn take_until_alt<F>(&mut self, start: usize, mut end: usize,  mut terminate: F)
        -> (&'input str, usize)
        where F: FnMut(char) -> bool
    {
        while let Some((_, c, _)) = self.lookahead {
            if terminate(c) {
                let e = self.advance();
                end = e.unwrap().2;
                return (&self.input[start..end], end);
            } else if let Some((_, _, e)) = self.advance() {
                end = e;
            }
        } 
        (&self.input[start..end], end)
    }
    fn take_until_seen_twice<F>(&mut self, start: usize, mut end: usize,  mut terminate: F)
        -> (&'input str, usize)
        where F: FnMut(char) -> bool
    {
        //eprintln!("{:?}",self.input);
        let mut count = 0;
        while let Some((_, c, _)) = self.lookahead {
            if count == 2 {
                return (&self.input[start..end], end);
            }
            else if terminate(c) {
                count += 1;
            } 
            else if let Some((_, _, e)) = self.advance() {
                end = e;
            }
        } 
        (&self.input[start..end], end)
    }

    fn take_while<F>(&mut self, start: usize, end: usize, mut keep_going: F)
        -> (&'input str, usize)
        where F: FnMut(char) -> bool,
    {
        self.take_until(start, end, |c| !keep_going(c))
    }

    fn single_quote(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        let (word, end) = self.take_until_alt(start, end, |c| c == '\'');
        //self.advance();
        Ok((start, Token::Word(&word[0..]), end))
    }
    fn double_quote(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        let (word, end) = self.take_until_alt(start, end, |c| c == '"');
        //self.advance();
        Ok((start, Token::Word(&word[0..]), end))
    }

    fn subshell(&mut self, chr: char, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {

        match chr {
            '$' => {
                let (word, end) = self.take_until_alt(start, end, |c| c == ')');

                return Ok((start, Token::Word(&word[0..]), end));
            },
            '`' => {
                let (word, end) = self.take_until_alt(start, end, |c| c == '`');
                return Ok((start, Token::Word(&word[0..]), end));
            },
            _ => {
                unreachable!();
            }
        }
    }

    fn newline_list(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {
        while let Some((_, chr, _)) = self.lookahead {
            //eprintln!("chr: {}", chr);
            match chr {
                '\n' => self.advance(),
                _ => break,
            };
        }
        Ok((start, Token::NewlineList, end))
    }

    fn word(&mut self, start: usize, end: usize) -> Result<(usize, Token<'input>, usize), Error> {

        let (word, end) = self.take_while(start, end, is_word_continue);
        //eprintln!("word: {}", word);
        let token = match word {
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "elif" => Token::Elif,
            "fi" => Token::Fi,
            "do" => Token::Do,
            "done" => Token::Done,
            "case" => Token::Case,
            "esac" => Token::Esac,
            word => self.num_or_word(word),
        };
        
        Ok((start, token, end))
        
    }

    fn num_or_word(&self, word: &'input str) -> Token<'input> {
        if let Some((_,chr,_)) = self.lookahead {
            if chr == '<' || chr == '>' {
                match word.parse::<i32>() {
                    Ok(num) => return Token::Number(num),
                    Err(_) => return Token::Word(word),
                }
            }
        }
        Token::Word(word)
    }
}


fn is_word_start(chr: char) -> bool {
    //eprintln!("chr is_word_start: {}", chr);
    let result = match chr {
        'a'..='z' | 'A'..='Z' | '_' | '0'..='9' | '"' | '\''  => true,
        _ => is_word_continue(chr),
    };
    //eprintln!("result is_word_start: {}", result);
    result
}

fn is_word_continue(chr: char) -> bool {
    match chr {
        ';' | '&' | '|' | '(' | ')' | '{' | '}' | '<' | '>' | '!' | '$' | '`' | '*' | '=' => false,
        _ => !chr.is_whitespace(),
    }
}


