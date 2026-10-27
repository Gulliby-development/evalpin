use crate::text::{self, Failure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Row {
    pub id: String,
    pub passed: usize,
    pub total: usize,
}

pub fn write(rows: &[Row]) -> String {
    let mut output = String::from("evalpin-results 1\n");
    for row in rows {
        output.push_str(&format!("{} {} {}\n", row.id, row.passed, row.total));
    }
    output
}

pub fn read(lines: &[String], file: &str) -> Result<Vec<Row>, Failure> {
    let Some(header) = lines.first() else {
        return Err(line_failure(file, 1, "not an evalpin results file"));
    };
    if header != "evalpin-results 1" {
        return Err(line_failure(file, 1, "not an evalpin results file"));
    }

    let mut rows = Vec::new();
    let mut ids = Vec::new();
    for (index, line) in lines.iter().enumerate().skip(1) {
        let line_number = index + 1;
        let mut fields = line.split(' ');
        let Some(id) = fields.next() else {
            return Err(line_failure(
                file,
                line_number,
                "line is not ID PASSED TOTAL",
            ));
        };
        let Some(passed_text) = fields.next() else {
            return Err(line_failure(
                file,
                line_number,
                "line is not ID PASSED TOTAL",
            ));
        };
        let Some(total_text) = fields.next() else {
            return Err(line_failure(
                file,
                line_number,
                "line is not ID PASSED TOTAL",
            ));
        };
        if fields.next().is_some()
            || id.is_empty()
            || passed_text.is_empty()
            || total_text.is_empty()
        {
            return Err(line_failure(
                file,
                line_number,
                "line is not ID PASSED TOTAL",
            ));
        }

        if !text::is_id(id) {
            return Err(line_failure(
                file,
                line_number,
                &format!("case {} is not valid", text::name(id)),
            ));
        }

        let passed = passed_text.parse::<usize>();
        let total = total_text.parse::<usize>();
        let counts_are_valid = text::is_digits(passed_text, 6)
            && text::is_digits(total_text, 6)
            && matches!((&passed, &total), (Ok(passed), Ok(total)) if *total >= 1 && *passed <= *total);
        if !counts_are_valid {
            return Err(line_failure(
                file,
                line_number,
                &format!(
                    "counts {} of {} are not valid",
                    text::name(passed_text),
                    text::name(total_text)
                ),
            ));
        }

        if ids.iter().any(|seen| seen == id) {
            return Err(line_failure(
                file,
                line_number,
                &format!("case {} given twice", text::name(id)),
            ));
        }

        let (Ok(passed), Ok(total)) = (passed, total) else {
            return Err(line_failure(
                file,
                line_number,
                &format!(
                    "counts {} of {} are not valid",
                    text::name(passed_text),
                    text::name(total_text)
                ),
            ));
        };
        ids.push(id.to_owned());
        rows.push(Row {
            id: id.to_owned(),
            passed,
            total,
        });
    }

    Ok(rows)
}

fn line_failure(file: &str, line: usize, message: &str) -> Failure {
    Failure(format!("{}:{}: {}", text::name(file), line, message))
}
