use nom::{
    IResult, Offset, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_until, take_while, take_while1},
    character::complete::{char, multispace0},
    combinator::{opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, preceded},
};

#[derive(Debug, Clone)]
pub struct DataAttr<'a> {
    pub name: &'a str,
    pub value: Option<&'a str>,
    pub start_pos: usize,
}

fn template_double(input: &str) -> IResult<&str, &str> {
    recognize(delimited(tag("{{"), take_until("}}"), tag("}}"))).parse(input)
}

fn template_block(input: &str) -> IResult<&str, &str> {
    recognize(delimited(tag("{%"), take_until("%}"), tag("%}"))).parse(input)
}

fn attr_component(input: &str) -> IResult<&str, &str> {
    alt((
        template_double,
        template_block,
        take_while1(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
    ))
    .parse(input)
}

fn attr_key(input: &str) -> IResult<&str, &str> {
    recognize(many1(attr_component)).parse(input)
}

fn attr_value(input: &str) -> IResult<&str, &str> {
    preceded(
        (multispace0, char('='), multispace0),
        alt((
            delimited(char('"'), is_not("\""), char('"')),
            delimited(char('\''), is_not("'"), char('\'')),
            take_while(|c: char| !c.is_whitespace() && c != '>' && c != '/'),
        )),
    )
    .parse(input)
}

const DATA: &str = "data-";

fn data_attr(input: &str) -> IResult<&str, (&str, Option<&str>)> {
    let (remaining, (_, name, value)) = (tag(DATA), attr_key, opt(attr_value)).parse(input)?;
    Ok((remaining, (name, value)))
}

pub fn find_all_data_attrs<'a>(input: &'a str) -> Vec<DataAttr<'a>> {
    fn search_and_parse<'a>(input: &'a str) -> IResult<&'a str, Vec<DataAttr<'a>>> {
        many0(|i| -> IResult<&'a str, DataAttr<'a>> {
            let (remaining, _) = take_until(DATA)(i)?;
            let offset = input.offset(remaining);

            let (rest, (name, value)) = data_attr(remaining)?;
            let attr = DataAttr {
                name,
                value,
                start_pos: offset + DATA.len(),
            };

            Ok((rest, attr))
        })
        .parse(input)
    }

    let (_, data_attrs) = search_and_parse(input).unwrap_or((input, Vec::new()));
    data_attrs
}
