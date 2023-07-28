use std::iter::Peekable;

use super::game_interface::{BuiltinType, GameInterface, Type, StructFields, StructField, EnumVariants, EnumVariant, FunctionSignature};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenData {
    OpenParen, CloseParen,
    OpenBracket, CloseBracket,
    OpenBrace, CloseBrace,

    Identifier(String), Number(i64), BuiltinType(BuiltinType),

    Colon, Comma, Semicolon, Equals, Arrow,

    Type, Function, Enum, Struct
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    data: TokenData,
    line: usize,
    col: usize
}

#[derive(Clone)]
struct Tokenizer {
    input: String,
    pos: usize,

    line: usize,
    col: usize,

    error: bool
}

impl Tokenizer {
    pub fn new(input: String) -> Tokenizer {
        Tokenizer {
            input,
            pos: 0,

            line: 1,
            col: 1,

            error: false
        }
    }

    fn absorb_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input.chars().nth(self.pos).unwrap();
            if !c.is_whitespace() {
                break;
            }

            self.pos += 1;

            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn get_char(&self, pos: usize) -> Option<char> {
        self.input.chars().nth(pos)
    }
}

impl Iterator for Tokenizer {
    type Item = Result<Token, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error {
            return None;
        }

        self.absorb_whitespace();
        if self.pos >= self.input.len() {
            return None;
        }

        let start_col = self.col;

        let mut end = self.pos + 1;

        let c = self.input.chars().nth(self.pos).unwrap();

        let token = match c {
            '(' => TokenData::OpenParen,
            ')' => TokenData::CloseParen,
            '[' => TokenData::OpenBracket,
            ']' => TokenData::CloseBracket,
            '{' => TokenData::OpenBrace,
            '}' => TokenData::CloseBrace,
            ':' => TokenData::Colon,
            ',' => TokenData::Comma,
            ';' => TokenData::Semicolon,
            '=' => TokenData::Equals,
            '-' => {
                end += 1;
                match self.get_char(end - 1) {
                    Some('>') => TokenData::Arrow,
                    Some(c) => {
                        self.error = true;
                        return Some(Err(format!("Unexpected character '{}' after '-'. Line {}, Col {}", c, self.line, self.col)));
                    },
                    None => {
                        self.error = true;
                        return Some(Err(format!("Unexpected EOF after '-'. Line {}, Col {}", self.line, self.col)));
                    }
                }
            },
            '0'..='9' => {
                while end < self.input.len() {
                    let c = self.input.chars().nth(end).unwrap();
                    if !c.is_digit(10) {
                        break;
                    }
                    end += 1;
                }
                let num = match self.input[self.pos..end].parse::<i64>() {
                    Ok(num) => num,
                    Err(e) => {
                        self.error = true;
                        return Some(Err(format!("Failed to parse number: {}. Number may be too large", e)));
                    }
                };
                
                TokenData::Number(num)
            },
            'a'..='z' | 'A'..='Z' | '_' => {
                while end < self.input.len() {
                    let c = self.input.chars().nth(end).unwrap();
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    end += 1;
                }
                let ident = self.input[self.pos..end].to_string();
                match ident.as_str() {
                    "type" => TokenData::Type,
                    "function" => TokenData::Function,
                    "enum" => TokenData::Enum,
                    "struct" => TokenData::Struct,
                    "u8" => TokenData::BuiltinType(BuiltinType::U8),
                    "u16" => TokenData::BuiltinType(BuiltinType::U16),
                    "u32" => TokenData::BuiltinType(BuiltinType::U32),
                    "u64" => TokenData::BuiltinType(BuiltinType::U64),
                    "i8" => TokenData::BuiltinType(BuiltinType::I8),
                    "i16" => TokenData::BuiltinType(BuiltinType::I16),
                    "i32" => TokenData::BuiltinType(BuiltinType::I32),
                    "i64" => TokenData::BuiltinType(BuiltinType::I64),
                    "f32" => TokenData::BuiltinType(BuiltinType::F32),
                    "f64" => TokenData::BuiltinType(BuiltinType::F64),
                    "bool" => TokenData::BuiltinType(BuiltinType::Bool),
                    "str" => TokenData::BuiltinType(BuiltinType::Str),
                    _ => TokenData::Identifier(ident)
                }
            },
            _ => {
                self.error = true;
                return Some(Err(format!("Unexpected character '{}'", c)));
            }
        };

        self.col += end - self.pos;

        self.pos = end;

        Some(Ok(Token {
            data: token,
            line: self.line,
            col: start_col
        }))
    }
}

#[derive(Clone)]
pub struct Parser {
    tokens: Peekable<Tokenizer>,

    res: GameInterface
}

impl Parser {
    pub fn new(input: String, name: String) -> Parser {
        Parser {
            tokens: Tokenizer::new(input).peekable(),

            res: GameInterface {
                name,
                types: Vec::new(),
                functions: Vec::new()
            }
        }
    }

    pub fn parse(mut self) -> Result<GameInterface, String> {
        while self.tokens.peek().is_some() {
            self.parse_top_level()?;
        }

        Ok(self.res)
    }

    fn consume(&mut self, token: TokenData) -> Result<(), String> {
        let next = self.tokens.next().unwrap()?;
        if next.data != token {
            Err(format!("Expected token {:?}, got {:?}", token, next))
        } else {
            Ok(())
        }
    }

    fn next(&mut self) -> Result<Token, String> {
        match self.tokens.next() {
            Some(Ok(token)) => Ok(token),
            Some(Err(e)) => Err(e),
            None => Err("Unexpected EOF".to_string())
        }
    }

    fn peek(&mut self) -> Result<Token, String> {
        match self.tokens.peek() {
            Some(Ok(token)) => Ok(token.clone()),
            Some(Err(e)) => Err(e.clone()),
            None => Err("Unexpected EOF".to_string())
        }
    }

    fn parse_top_level(&mut self) -> Result<(), String> {
        match self.next()? {
            Token {data: TokenData::Type, ..} => self.parse_type_def()?,
            Token {data: TokenData::Function, ..} => self.parse_function()?,
            token => {
                return Err(format!("Unexpected token {:?} at top level", token));
            }
        };

        self.consume(TokenData::Semicolon)?;

        Ok(())
    }

    fn parse_type_def(&mut self) -> Result<(), String> {
        let name = match self.tokens.next().unwrap()? {
            Token {data: TokenData::Identifier(name), ..} => name,
            token => {
                return Err(format!("Expected identifier, got {:?}", token));
            }
        };

        self.consume(TokenData::Equals)?;

        let ty = self.parse_type_expr()?;

        self.res.types.push((name, ty));

        Ok(())
    }

    fn parse_type_expr(&mut self) -> Result<Type, String> {
        let token = self.tokens.next().unwrap()?;
        match token {
            Token{data: TokenData::BuiltinType(ty), ..} => Ok(Type::Builtin(ty)),
            Token{data: TokenData::Identifier(name), ..} => Ok(Type::NamedType(name)),
            Token{data: TokenData::Struct, ..} => Ok(Type::Struct(Box::new(self.parse_struct()?))),
            Token{data: TokenData::Enum, ..} => Ok(Type::Enum(Box::new(self.parse_enum()?))),
            Token{data: TokenData::OpenBracket, ..} => Ok(self.parse_array()?),
            token => {
                Err(format!("Expected type expression, got {:?}", token))
            }
        }
    }

    fn parse_direct_type_expr(&mut self) -> Result<Type, String> {
        let token = self.tokens.next().unwrap()?;
        match token {
            Token{data: TokenData::BuiltinType(ty), ..} => Ok(Type::Builtin(ty)),
            Token{data: TokenData::Identifier(name), ..} => Ok(Type::NamedType(name)),
            token => {
                Err(format!("Expected type expression, got {:?}", token))
            }
        }
    }

    fn parse_struct(&mut self) -> Result<StructFields, String> {
        let mut res = Vec::new();

        self.consume(TokenData::OpenBrace)?;

        while self.peek()?.data != TokenData::CloseBrace {
            let name = match self.tokens.next().unwrap()? {
                Token {data: TokenData::Identifier(name), ..} => name,
                token => {
                    return Err(format!("Expected identifier, got {:?}", token));
                }
            };

            self.consume(TokenData::Colon)?;

            let ty = self.parse_direct_type_expr()?;

            res.push(StructField {
                name,
                ty
            });

            if self.peek()?.data == TokenData::Comma {
                self.consume(TokenData::Comma)?;
            } else {
                break;
            }
        }

        self.consume(TokenData::CloseBrace)?;

        Ok(res)
    }

    fn parse_enum(&mut self) -> Result<EnumVariants, String> {
        let mut res = Vec::new();

        self.consume(TokenData::OpenBrace)?;

        while self.peek()?.data != TokenData::CloseBrace {
            let name = match self.tokens.next().unwrap()? {
                Token{data: TokenData::Identifier(name), ..} => name,
                token => {
                    return Err(format!("Expected identifier, got {:?}", token));
                }
            };

            let fields = if self.peek()?.data == TokenData::OpenBrace {
                self.parse_struct()?
            } else {
                Vec::new()
            };

            res.push(EnumVariant {
                name,
                types: fields
            });

            if self.peek()?.data == TokenData::Comma {
                self.consume(TokenData::Comma)?;
            } else {
                break;
            }
        }

        self.consume(TokenData::CloseBrace)?;

        Ok(res)
    }

    fn parse_array(&mut self) -> Result<Type, String> {
        let ty = self.parse_type_expr()?;

        if let TokenData::Semicolon = self.peek()?.data {
            self.consume(TokenData::Semicolon)?;

            let size = match self.next()? {
                Token{data: TokenData::Number(size), ..} => size,
                token => {
                    return Err(format!("Expected integer, got {:?}", token));
                }
            };

            self.consume(TokenData::CloseBracket)?;
            
            Ok(Type::Array(Box::new(ty), size as _))
        } else {
            self.consume(TokenData::CloseBracket)?;

            Ok(Type::DynamicArray(Box::new(ty)))
        }
    }

    fn parse_function(&mut self) -> Result<(), String> {
        let name = match self.tokens.next().unwrap()? {
            Token{data: TokenData::Identifier(name), ..} => name,
            token => {
                return Err(format!("Expected identifier, got {:?}", token));
            }
        };

        self.consume(TokenData::Equals)?;
        self.consume(TokenData::OpenParen)?;

        let mut args = Vec::new();

        while self.peek()?.data != TokenData::CloseParen {
            let name = match self.tokens.next().unwrap()? {
                Token{data: TokenData::Identifier(name), ..} => name,
                token => {
                    return Err(format!("Expected identifier, got {:?}", token));
                }
            };

            self.consume(TokenData::Colon)?;

            let ty = self.parse_type_expr()?;

            args.push((name, ty));

            if self.peek()?.data == TokenData::Comma {
                self.consume(TokenData::Comma)?;
            } else {
                break;
            }
        }

        self.consume(TokenData::CloseParen)?;

        let ret_ty = if self.peek()?.data == TokenData::Arrow {
            self.consume(TokenData::Arrow)?;

            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.res.functions.push((
            name,
            FunctionSignature {
                args,
                ret: ret_ty
            }
        ));

        Ok(())
    }
}

pub fn parse_game_interface(source: &str, name: String) -> Result<GameInterface, String> {
    let parser = Parser::new(source.to_string(), name);
    parser.parse()
}