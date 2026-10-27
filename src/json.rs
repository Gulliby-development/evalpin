#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

pub fn parse(text: &str) -> Option<Json> {
    let mut parser = Parser { text, position: 0 };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.position == text.len() {
        Some(value)
    } else {
        None
    }
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
}

impl Parser<'_> {
    fn byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.position).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        self.whitespace();
        match self.byte()? {
            b'n' => {
                self.keyword("null")?;
                Some(Json::Null)
            }
            b't' => {
                self.keyword("true")?;
                Some(Json::Bool(true))
            }
            b'f' => {
                self.keyword("false")?;
                Some(Json::Bool(false))
            }
            b'"' => self.string().map(Json::Str),
            b'[' => self.array(depth),
            b'{' => self.object(depth),
            b'-' | b'0'..=b'9' => self.number().map(Json::Num),
            _ => None,
        }
    }

    fn keyword(&mut self, word: &str) -> Option<()> {
        let remaining = self.text.get(self.position..)?;
        if !remaining.starts_with(word) {
            return None;
        }
        self.position += word.len();
        Some(())
    }

    fn array(&mut self, depth: usize) -> Option<Json> {
        if depth >= 20 {
            return None;
        }
        self.position += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.byte() == Some(b']') {
            self.position += 1;
            return Some(Json::Arr(values));
        }

        loop {
            values.push(self.value(depth + 1)?);
            self.whitespace();
            match self.byte()? {
                b',' => {
                    self.position += 1;
                }
                b']' => {
                    self.position += 1;
                    return Some(Json::Arr(values));
                }
                _ => return None,
            }
        }
    }

    fn object(&mut self, depth: usize) -> Option<Json> {
        if depth >= 20 {
            return None;
        }
        self.position += 1;
        self.whitespace();
        let mut pairs = Vec::new();
        if self.byte() == Some(b'}') {
            self.position += 1;
            return Some(Json::Obj(pairs));
        }

        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            if self.byte()? != b':' {
                return None;
            }
            self.position += 1;
            let value = self.value(depth + 1)?;
            pairs.push((key, value));
            self.whitespace();
            match self.byte()? {
                b',' => {
                    self.position += 1;
                }
                b'}' => {
                    self.position += 1;
                    return Some(Json::Obj(pairs));
                }
                _ => return None,
            }
        }
    }

    fn number(&mut self) -> Option<String> {
        let start = self.position;
        if self.byte() == Some(b'-') {
            self.position += 1;
        }

        match self.byte()? {
            b'0' => self.position += 1,
            b'1'..=b'9' => {
                self.position += 1;
                while matches!(self.byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return None,
        }

        if self.byte() == Some(b'.') {
            self.position += 1;
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return None;
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return None;
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        self.text.get(start..self.position).map(str::to_owned)
    }

    fn string(&mut self) -> Option<String> {
        if self.byte()? != b'"' {
            return None;
        }
        self.position += 1;
        let mut result = String::new();

        loop {
            let character = self.text.get(self.position..)?.chars().next()?;
            match character {
                '"' => {
                    self.position += 1;
                    return Some(result);
                }
                '\\' => {
                    self.position += 1;
                    let escape = self.byte()?;
                    self.position += 1;
                    match escape {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{8}'),
                        b'f' => result.push('\u{c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let scalar = if (0xd800..=0xdbff).contains(&first) {
                                if self.text.get(self.position..)?.starts_with("\\u") {
                                    self.position += 2;
                                } else {
                                    return None;
                                }
                                let second = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return None;
                                }
                                0x10000
                                    + ((u32::from(first) - 0xd800) << 10)
                                    + (u32::from(second) - 0xdc00)
                            } else if (0xdc00..=0xdfff).contains(&first) {
                                return None;
                            } else {
                                u32::from(first)
                            };
                            result.push(char::from_u32(scalar)?);
                        }
                        _ => return None,
                    }
                }
                control if control <= '\u{1f}' => return None,
                other => {
                    result.push(other);
                    self.position += other.len_utf8();
                }
            }
        }
    }

    fn hex4(&mut self) -> Option<u16> {
        let mut value = 0u16;
        for _ in 0..4 {
            let digit = match self.byte()? {
                b'0'..=b'9' => u16::from(self.byte()? - b'0'),
                b'a'..=b'f' => u16::from(self.byte()? - b'a' + 10),
                b'A'..=b'F' => u16::from(self.byte()? - b'A' + 10),
                _ => return None,
            };
            value = (value << 4) | digit;
            self.position += 1;
        }
        Some(value)
    }
}
