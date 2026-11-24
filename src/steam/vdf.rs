//! Reads the text `KeyValues` format of Steam files (`.vdf`, `.acf`): each key has a text value
//! in quotes or a block of keys in `{ }`. In a quoted text, `\\` is `\` and `\"` is `"`. A line
//! comment starts with `//`.
//!
//! A damaged file gives a `Damage` with its line number. Blocks deeper than `MAX_DEPTH` count as
//! damage: the tree is freed by recursion, so a very deep tree could overflow the stack. Steam
//! files are less than 10 blocks deep.

use std::fmt;

/// The deepest block that `parse` reads.
const MAX_DEPTH: usize = 64;

/// A block of keys, in file order. Steam reads keys with no regard to case, so `get` does too.
#[derive(Debug, Default, PartialEq)]
pub struct Block(Vec<(String, Node)>);

/// The value of one key.
#[derive(Debug, PartialEq)]
pub enum Node {
    Text(String),
    Block(Block),
}

/// What is wrong in a damaged file, and on which line.
#[derive(Debug, PartialEq)]
pub struct Damage {
    pub line: usize,
    pub problem: Problem,
}

#[derive(Debug, PartialEq)]
pub enum Problem {
    /// A text in quotes has no closing quote.
    OpenQuote,

    /// A block has no closing `}`.
    OpenBlock { key: String },

    /// A `}` closes no block.
    ExtraClose,

    /// A key has no value before the end of its block.
    NoValue { key: String },

    /// A `{` has no key before it.
    BlockWithoutKey,

    /// The blocks are more than `MAX_DEPTH` deep.
    TooDeep,
}

/// Reads the text of a Steam file into its top block.
pub fn parse(text: &str) -> Result<Block, Damage> {
    let mut tokens = Tokens {
        rest: text,
        line: 1,
    };
    // The open blocks: each with its key, the line of its `{`, and its keys so far.
    let mut open: Vec<(String, usize, Block)> = Vec::new();
    let mut top = Block::default();
    let mut key: Option<(String, usize)> = None;
    while let Some((token, line)) = tokens.next_token()? {
        let current = open.last_mut().map_or(&mut top, |(_, _, block)| block);
        match token {
            Token::Text(text) => match key.take() {
                Some((name, _)) => current.0.push((name, Node::Text(text))),
                None => key = Some((text, line)),
            },
            Token::Open => {
                let (name, _) = key.take().ok_or(Damage {
                    line,
                    problem: Problem::BlockWithoutKey,
                })?;
                if open.len() == MAX_DEPTH {
                    return Err(Damage {
                        line,
                        problem: Problem::TooDeep,
                    });
                }
                open.push((name, line, Block::default()));
            }
            Token::Close => {
                if let Some((name, line)) = key.take() {
                    return Err(Damage {
                        line,
                        problem: Problem::NoValue { key: name },
                    });
                }
                let (name, _, block) = open.pop().ok_or(Damage {
                    line,
                    problem: Problem::ExtraClose,
                })?;
                let parent = open.last_mut().map_or(&mut top, |(_, _, block)| block);
                parent.0.push((name, Node::Block(block)));
            }
        }
    }
    if let Some((name, line)) = key {
        return Err(Damage {
            line,
            problem: Problem::NoValue { key: name },
        });
    }
    if let Some((name, line, _)) = open.pop() {
        return Err(Damage {
            line,
            problem: Problem::OpenBlock { key: name },
        });
    }
    Ok(top)
}

/// Accessors
impl Block {
    /// The value of the first key named `key`, with no regard to case.
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.0
            .iter()
            .find_map(|(name, node)| name.eq_ignore_ascii_case(key).then_some(node))
    }

    /// The block of the first key named `key`.
    pub fn block(&self, key: &str) -> Option<&Block> {
        match self.get(key)? {
            Node::Block(block) => Some(block),
            Node::Text(_) => None,
        }
    }

    /// The text of the first key named `key`.
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Node::Text(text) => Some(text),
            Node::Block(_) => None,
        }
    }

    /// The block at the end of the key path `keys`, for example `["Software", "Valve"]`.
    pub fn block_at(&self, keys: &[&str]) -> Option<&Block> {
        keys.iter().try_fold(self, |block, key| block.block(key))
    }

    /// The blocks of this block, in file order, with their keys.
    pub fn blocks(&self) -> impl Iterator<Item = (&str, &Block)> {
        self.0.iter().filter_map(|(name, node)| match node {
            Node::Block(block) => Some((name.as_str(), block)),
            Node::Text(_) => None,
        })
    }
}

impl fmt::Display for Damage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.problem {
            Problem::OpenQuote => write!(f, "a text in quotes has no closing quote"),
            Problem::OpenBlock { key } => write!(f, "the block \"{key}\" has no closing }}"),
            Problem::ExtraClose => write!(f, "a }} closes no block"),
            Problem::NoValue { key } => write!(f, "the key \"{key}\" has no value"),
            Problem::BlockWithoutKey => write!(f, "a {{ has no key before it"),
            Problem::TooDeep => write!(f, "the blocks are more than {MAX_DEPTH} deep"),
        }
    }
}

// region:    --- Support

enum Token {
    Text(String),
    Open,
    Close,
}

/// The tokens of a file, with the line where each one starts.
struct Tokens<'a> {
    rest: &'a str,
    line: usize,
}

impl Tokens<'_> {
    fn next_token(&mut self) -> Result<Option<(Token, usize)>, Damage> {
        self.skip_space_and_comments();
        let mut chars = self.rest.chars();
        let Some(first) = chars.next() else {
            return Ok(None);
        };
        let line = self.line;
        let token = match first {
            '{' => {
                self.rest = chars.as_str();
                Token::Open
            }
            '}' => {
                self.rest = chars.as_str();
                Token::Close
            }
            '"' => {
                self.rest = chars.as_str();
                Token::Text(self.quoted(line)?)
            }
            _ => Token::Text(self.bare()),
        };
        Ok(Some((token, line)))
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            let trimmed = self.rest.trim_start();
            self.count_lines(self.rest.len() - trimmed.len());
            self.rest = trimmed;
            if !self.rest.starts_with("//") {
                return;
            }
            let end = self.rest.find('\n').unwrap_or(self.rest.len());
            self.rest = self.rest.get(end..).unwrap_or_default();
        }
    }

    /// The text after an opening quote, up to the closing quote, with its escapes read.
    fn quoted(&mut self, line: usize) -> Result<String, Damage> {
        let mut text = String::new();
        let mut chars = self.rest.char_indices();
        while let Some((at, c)) = chars.next() {
            match c {
                '"' => {
                    self.count_lines(at);
                    self.rest = self.rest.get(at + 1..).unwrap_or_default();
                    return Ok(text);
                }
                '\\' => match chars.next() {
                    Some((_, escaped @ ('\\' | '"'))) => text.push(escaped),
                    Some((_, 'n')) => text.push('\n'),
                    Some((_, 't')) => text.push('\t'),
                    Some((_, other)) => {
                        text.push('\\');
                        text.push(other);
                    }
                    None => text.push('\\'),
                },
                _ => text.push(c),
            }
        }
        Err(Damage {
            line,
            problem: Problem::OpenQuote,
        })
    }

    /// A text without quotes: up to a space, a quote, or a brace.
    fn bare(&mut self) -> String {
        let end = self
            .rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '{' | '}'))
            .unwrap_or(self.rest.len());
        let (text, rest) = self.rest.split_at(end);
        self.rest = rest;
        text.to_owned()
    }

    /// Counts the line ends in the next `len` bytes of `rest`.
    fn count_lines(&mut self, len: usize) {
        let skipped = self.rest.get(..len).unwrap_or_default();
        self.line += skipped.matches('\n').count();
    }
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keys_blocks_and_escapes() {
        // -- Setup & Fixtures
        // A part of localconfig.vdf of Steam 2026, with a comment added.
        let text = "\"UserLocalConfigStore\"\n{\n\t// apps\n\t\"apps\"\n\t{\n\t\t\"1874880\"\n\t\t{\n\
                    \t\t\t\"LaunchOptions\"\t\t\"-addonDownloadDir \\\"D:\\\\DownloadedMods\\\"\"\n\
                    \t\t}\n\t}\n}\n";

        // -- Exec
        let top = parse(text).unwrap();

        // -- Check
        let app = top
            .block_at(&["userlocalconfigstore", "Apps", "1874880"])
            .unwrap();
        assert_eq!(
            app.text("launchoptions"),
            Some(r#"-addonDownloadDir "D:\DownloadedMods""#)
        );
    }

    #[test]
    fn reads_a_text_without_quotes() {
        // -- Exec
        let top = parse("\"a\" { key value }").unwrap();

        // -- Check
        assert_eq!(top.block("a").unwrap().text("key"), Some("value"));
    }

    #[test]
    fn gives_the_line_of_the_damage_in_a_damaged_file() {
        // -- Exec & Check
        for (text, line, problem) in [
            ("\"a\"\n{\n\"b\" \"c", 3, Problem::OpenQuote),
            (
                "\"a\"\n{\n\"b\" \"c\"\n",
                2,
                Problem::OpenBlock {
                    key: "a".to_owned(),
                },
            ),
            ("\"a\" \"b\"\n}", 2, Problem::ExtraClose),
            (
                "\"a\"\n{\n\"b\"\n}",
                3,
                Problem::NoValue {
                    key: "b".to_owned(),
                },
            ),
            (
                "\"a\"",
                1,
                Problem::NoValue {
                    key: "a".to_owned(),
                },
            ),
            ("{ }", 1, Problem::BlockWithoutKey),
        ] {
            assert_eq!(parse(text), Err(Damage { line, problem }), "{text}");
        }
    }

    #[test]
    fn gives_an_error_not_a_stack_overflow_for_very_deep_blocks() {
        // -- Setup & Fixtures
        let text = |depth| format!("{}{}", "\"a\" {".repeat(depth), "}".repeat(depth));

        // -- Exec & Check
        assert!(parse(&text(MAX_DEPTH)).is_ok());
        assert_eq!(
            parse(&text(100_000)),
            Err(Damage {
                line: 1,
                problem: Problem::TooDeep
            })
        );
    }
}

// endregion: --- Tests
