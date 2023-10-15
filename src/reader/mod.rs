use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};

pub mod tiny_v2;
pub mod tiny_v2_diff;

fn try_read<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<String> {
    if let Some(x) = iter.next() {
        if x.is_empty() {
            bail!("Entry may not be empty")
        } else {
            Ok(x.to_owned())
        }
    } else {
        bail!("No item given")
    }
}
fn try_read_optional<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Option<String>> {
    if let Some(x) = iter.next() {
        if x.is_empty() {
            Ok(None)
        } else {
            Ok(Some(x.to_owned()))
        }
    } else {
        Ok(None)
    }
}

pub trait ParseEntry {
    fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> where Self: Sized;
}
pub trait SetDoc<J> {
    fn set_doc(&mut self, doc: J);
}

pub trait AddMember<M> {
    fn add_member(&mut self, member: M);
}

#[derive(Debug)]
pub struct Parse<D, C, F, M, P, J> {
    output: D,
    class: Option<C>,
    field: Option<F>,
    method: Option<M>,
    parameter: Option<P>,
    j: Option<J>,
}

impl<D, C, F, M, P, J> Parse<D, C, F, M, P, J>
where
    D: AddMember<C>,
    C: ParseEntry + SetDoc<J> + AddMember<F> + AddMember<M>,
    F: ParseEntry + SetDoc<J>,
    M: ParseEntry + SetDoc<J> + AddMember<P>,
    P: ParseEntry + SetDoc<J>,
    J: ParseEntry,
{
    fn set_class(&mut self, class: Option<C>) {
        if let Some(c) = std::mem::replace(&mut self.class, class) {
            self.output.add_member(c);
        }
    }
    fn set_field(&mut self, field: Option<F>) -> Result<()> {
        if let Some(f) = std::mem::replace(&mut self.field, field) {
            self.class.as_mut()
                .ok_or_else(|| anyhow!("cannot read field mapping: not in a class?"))?
                .add_member(f)
        }
        Ok(())
    }
    fn set_method(&mut self, method: Option<M>) -> Result<()> {
        if let Some(m) = std::mem::replace(&mut self.method, method) {
            self.class.as_mut()
                .ok_or_else(|| anyhow!("cannot read method mapping: not in a class?"))?
                .add_member(m);
        }
        Ok(())
    }
    fn set_parameter(&mut self, parameter: Option<P>) -> Result<()> {
        if let Some(p) = std::mem::replace(&mut self.parameter, parameter) {
            self.method.as_mut()
                .ok_or_else(|| anyhow!("cannot read parameter mapping: not in a method?"))?
                .add_member(p);
        }
        Ok(())
    }

    fn parse_line(&mut self, line: String) -> Result<()> {
        let mut iter = line.split('\t')
            .peekable();

        let idents = {
            let mut x = 0usize;
            while iter.next_if(|x| x.is_empty()).is_some() {
                x += 1;
            }
            x
        };

        match (idents, iter.next()) {
            (1, Some("c")) => { // class comment
                if let Some(ref mut class) = self.class {
                    class.set_doc(J::from_iter(&mut iter)?);
                } else {
                    bail!("cannot read class javadocs: not in a class?");
                }
            },
            (2, Some("c")) => { // field/method comment
                if let Some(ref mut field) = self.field {
                    field.set_doc(J::from_iter(&mut iter)?);
                } else if let Some(ref mut method) = self.method {
                    method.set_doc(J::from_iter(&mut iter)?);
                } else {
                    bail!("cannot read field/method javadocs: not in field or method?");
                }
            },
            (3, Some("c")) => { // parameter comment
                if let Some(ref mut parameter) = self.parameter {
                    parameter.set_doc(J::from_iter(&mut iter)?);
                } else {
                    bail!("cannot read parameter javadocs: not in a parameter?");
                }
            },
            (0, Some("c")) => { // class
                self.set_parameter(None)?;
                self.set_method(None)?;
                self.set_field(None)?;
                self.set_class(Some(C::from_iter(&mut iter)?));
            },
            (1, Some("f")) => {
                self.set_parameter(None)?;
                self.set_method(None)?;
                self.set_field(Some(F::from_iter(&mut iter)?))?;
            },
            (1, Some("m")) => {
                self.set_parameter(None)?;
                self.set_method(Some(M::from_iter(&mut iter)?))?;
                self.set_field(None)?;
            },
            (2, Some("p")) => {
                self.set_parameter(Some(P::from_iter(&mut iter)?))?;
                self.set_field(None)?;
            },
            s => bail!("unknown mapping target {s:?}: {:?}", iter.collect::<Vec<_>>()),
        }
        if iter.next().is_none() {
            Ok(())
        } else {
            bail!("line doesn't end")
        }
    }
}

pub fn parse<D, C, F, M, P, J>(reader: impl Read) -> Result<D>
where
    D: ParseEntry + AddMember<C>,
    C: ParseEntry + SetDoc<J> + AddMember<F> + AddMember<M>,
    F: ParseEntry + SetDoc<J>,
    M: ParseEntry + SetDoc<J> + AddMember<P>,
    P: ParseEntry + SetDoc<J>,
    J: ParseEntry,
{
    let mut lines = BufReader::new(reader)
        .lines()
        .enumerate();

    let header = lines.next()
        .ok_or_else(|| anyhow!("No Header"))?.1?;

    let mut header_fields = header.split("\t");

    if Some("tiny") != header_fields.next() {
        bail!("Not a tiny file");
    }
    if Some("2") != header_fields.next() {
        bail!("Tiny file of major version other than 2");
    }
    if Some("0") != header_fields.next() {
        bail!("Tiny file of minor version other than 0");
    }

    let mut parser: Parse<D, C, F, M, P, J> = Parse {
        output: D::from_iter(&mut header_fields)?,
        class: None,
        field: None,
        method: None,
        parameter: None,
        j: None,
    };

    for (line_number, line) in lines {
        parser.parse_line(line?)
            .with_context(|| anyhow!("In line {}", line_number + 1))?
    }

    parser.set_parameter(None)?;
    parser.set_method(None)?;
    parser.set_field(None)?;

    parser.set_class(None);

    Ok(parser.output)
}

