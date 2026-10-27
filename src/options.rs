use crate::text::{Failure, name};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub name: &'static str,
    pub repeat: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parsed {
    pub operands: Vec<String>,
    pub values: Vec<(String, String)>,
}

impl Parsed {
    pub fn value(&self, option: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(name, _)| name.as_str() == option)
            .map(|(_, value)| value.as_str())
    }

    pub fn all(&self, option: &str) -> Vec<&str> {
        self.values
            .iter()
            .filter(|(name, _)| name.as_str() == option)
            .map(|(_, value)| value.as_str())
            .collect()
    }
}

pub fn parse(args: &[String], specs: &[Spec], operands: usize) -> Result<Parsed, Failure> {
    let mut parsed = Parsed {
        operands: Vec::new(),
        values: Vec::new(),
    };
    let mut index = 0;

    while let Some(argument) = args.get(index) {
        if argument.starts_with("--") || argument.as_str() == "-k" {
            let Some(spec) = specs.iter().find(|spec| spec.name == argument.as_str()) else {
                return Err(Failure(format!("unknown option {}", name(argument))));
            };

            let Some(value) = args.get(index + 1) else {
                return Err(Failure(format!("{} needs a value", name(argument))));
            };

            if !spec.repeat
                && parsed
                    .values
                    .iter()
                    .any(|(option, _)| option.as_str() == argument.as_str())
            {
                return Err(Failure(format!("{} given twice", name(argument))));
            }

            parsed.values.push((argument.clone(), value.clone()));
            index += 2;
        } else {
            if parsed.operands.len() >= operands {
                return Err(Failure(format!("unexpected argument {}", name(argument))));
            }
            parsed.operands.push(argument.clone());
            index += 1;
        }
    }

    Ok(parsed)
}
