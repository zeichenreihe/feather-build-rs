// TODO: module doc
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use duke::tree::class::ClassNameSlice;
use duke::tree::method::MethodName;
use crate::action::extend_inner_class_names::ClassNameSliceExt;
use crate::enigma_file::enigma_line::EnigmaLine;
use crate::lines::WithMoreIdentIter;
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, JavadocMapping, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};
use crate::tree::names::Names;
use crate::tree::NodeInfo;

const CLASS: &str = "CLASS";
const FIELD: &str = "FIELD";
const METHOD: &str = "METHOD";
const PARAMETER: &str = "ARG";
const COMMENT: &str = "COMMENT";

/// Reads a enigma `.mapping` file, by opening the file given by the path.
///
/// This appends into the mappings.
///
/// Since the enigma format doesn't store namespaces, you need to give these to your mappings beforehand.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use std::path::Path;
/// use quill::tree::mappings::Mappings;
///
/// let path = Path::new("tests/read_file_input_enigma.txt");
/// let mut mappings = Mappings::from_namespaces(["namespaceA", "namespaceB"]).unwrap();
/// quill::enigma_file::read_file_into(path, &mut mappings).unwrap();
///
/// assert_eq!(mappings.classes.len(), 10);
/// ```
pub fn read_file_into(path: impl AsRef<Path>, mappings: &mut Mappings<2>) -> Result<()> {
	read_into(File::open(&path)?, mappings)
		.with_context(|| anyhow!("failed to read mappings file {:?} as enigma file", path.as_ref()))
}

#[allow(clippy::tabs_in_doc_comments)]
/// Reads the enigma format, from the given reader, appending into the mappings.
///
/// Since the enigma format doesn't store namespaces, you need to give these to your mappings beforehand.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use quill::tree::mappings::Mappings;
/// let string = "\
/// CLASS	A	B
/// 	FIELD a	b
/// 	METHOD	a	b	(LA;)V
/// 	COMMENT A multiline
/// 	COMMENT comment.
/// ";
///
/// let reader = &mut string.as_bytes();
/// let mut mappings = Mappings::from_namespaces(["namespaceA", "namespaceB"]).unwrap();
/// quill::enigma_file::read_into(reader, &mut mappings).unwrap();
///
/// assert_eq!(mappings.classes.len(), 1);
///
/// use duke::tree::class::ClassNameSlice;
/// assert_eq!(
///     mappings.classes.get(ClassNameSlice::from_str("A")).unwrap()
///         .javadoc.as_ref().map(|x| x.0.as_str()),
///     Some("A multiline\ncomment.")
/// );
/// ```
pub fn read_into(reader: impl Read, mappings: &mut Mappings<2>) -> Result<()> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<Option<EnigmaLine>> {
			EnigmaLine::new(line_number + 1, &line?)
		})
		.filter_map(|x| x.transpose())
		.peekable();

	WithMoreIdentIter::new(&mut lines).on_every_line(|iter, line| {
		match line.first_field.as_str() {
			CLASS => {
				// We use recursion here to parse classes contained in classes...
				fn parse_class(
					mappings: &mut Mappings<2>,
					iter: &mut WithMoreIdentIter<impl Iterator<Item=Result<EnigmaLine>>>,
					line: EnigmaLine,
					parent: Option<(&String, &String)>
				) -> Result<()> {
					let (src, dst) = match line.fields.as_slice() {
						[src] => (src, None),
						[src, mod_] if is_modifier(mod_) => (src, None),
						[src, dst] => (src, Some(dst)),
						[src, dst, _mod] => (src, Some(dst)),
						slice => bail!("illegal number of arguments ({}) for class mapping, expected 1-3, got {slice:?}", slice.len()),
					};

					let (src, dst) = if let Some((parent_src, parent_dst)) = parent {
						(format!("{parent_src}${src}"), dst.map(|dst| format!("{parent_dst}${dst}")))
					} else {
						(src.clone(), dst.cloned())
					};
					let parent_src = src.clone();
					let parent_dst = dst.clone().unwrap_or_else(|| parent_src.clone());
					let mut class = ClassNowodeMapping::new(ClassMapping {
						names: Names::try_from([Some(src.try_into()?), dst.map(|x| x.clone().try_into()).transpose()?])?,
					});

					iter.next_level().on_every_line(|iter, line| {
						match line.first_field.as_str() {
							CLASS => parse_class(mappings, iter, line, Some((&parent_src, &parent_dst))),
							FIELD => {
								let (src, dst, desc) = match line.fields.as_slice() {
									[src, desc] => (src, None, desc),
									[src, desc, mod_] if is_modifier(mod_) => (src, None, desc),
									[src, dst, desc] => (src, Some(dst), desc),
									[src, dst, desc, _mod] => (src, Some(dst), desc),
									slice => bail!("illegal number of arguments ({}) for field mapping, expected 2-4, got {slice:?}", slice.len()),
								};
								let field = FieldNowodeMapping::new(FieldMapping {
									desc: desc.to_owned().try_into()?,
									names: Names::try_from([Some(src.clone().try_into()?), dst.map(|x| x.clone().try_into()).transpose()?])?,
								});
								let field = class.add_field(field)?;

								iter.next_level().on_every_line(|_, line| {
									match line.first_field.as_str() {
										COMMENT => insert_comment(&mut field.javadoc, line),
										tag => bail!("unknown mapping target {tag:?} for inside field, allowed are: `COMMENT`"),
									}
								}).context("reading `FIELD` sub-sections")
							},
							METHOD => {
								let (src, dst, desc) = match line.fields.as_slice() {
									[src, desc] => (src, None, desc),
									[src, desc, mod_] if is_modifier(mod_) => (src, None, desc),
									[src, dst, desc] => (src, Some(dst), desc),
									[src, dst, desc, _mod] => (src, Some(dst), desc),
									slice => bail!("illegal number of arguments ({}) for method mapping, expected 2-4, got {slice:?}", slice.len()),
								};
								let method = MethodNowodeMapping::new(MethodMapping {
									desc: desc.to_owned().try_into()?,
									names: Names::try_from([Some(src.clone().try_into()?), dst.map(|x| x.clone().try_into()).transpose()?])?,
								});
								let method = class.add_method(method)?;

								iter.next_level().on_every_line(|iter, line| {
									match line.first_field.as_str() {
										PARAMETER => {
											let (raw_index, dst) = match line.fields.as_slice() {
												[raw_index, dst] => (raw_index, dst),
												slice => bail!("illegal number of arguments ({}) for parameter mapping, expected 2, got {slice:?}", slice.len()),
											};

											let index: usize = raw_index.parse()
												.with_context(|| anyhow!("illegal parameter index {raw_index:?}, index cannot be negative"))?;

											let parameter = ParameterNowodeMapping::new(ParameterMapping {
												index,
												names: [None, Some(dst.clone().try_into()?)].try_into()?,
											});
											let parameter = method.add_parameter(parameter)?;

											iter.next_level().on_every_line(|_, line| {
												match line.first_field.as_str() {
													COMMENT => insert_comment(&mut parameter.javadoc, line),
													tag => bail!("unknown mapping target {tag:?} for inside parameter, allowed are: `COMMENT`"),
												}
											}).context("reading `ARG` sub-sections")
										},
										COMMENT => insert_comment(&mut method.javadoc, line),
										tag => bail!("unknown mapping target {tag:?} for inside method, allowed are: `ARG`, `COMMENT`"),
									}
								}).context("reading `METHOD` sub-sections")
							},
							COMMENT => insert_comment(&mut class.javadoc, line),
							tag => bail!("unknown mapping target {tag:?} for inside class, allowed are: `CLASS`, `FIELD`, `METHOD`, `COMMENT`"),
						}
					}).context("reading `CLASS` sub-sections")?;

					// needs to be different because the closure above needs to modify `mappings` in the recursion
					mappings.add_class(class)?;

					Ok(())
				}
				parse_class(mappings, iter, line, None)
			},
			tag => bail!("unknown mapping target {tag:?} for inside root, allowed are: `CLASS`"),
		}
	}).context("reading lines")
}

fn is_modifier(s: &str) -> bool {
	const MODIFIER: &str = "ACC:";
	s.starts_with(MODIFIER)
}

fn insert_comment(javadoc: &mut Option<JavadocMapping>, line: EnigmaLine) -> Result<()> {
	let string = line.fields.join(" ");

	if let Some(javadoc) = javadoc {
		javadoc.0.push('\n');
		javadoc.0.push_str(&string);
	} else {
		*javadoc = Some(JavadocMapping(string));
	}
	Ok(())
}

mod enigma_line {
	use anyhow::{anyhow, Context, Result};
	use crate::lines::Line;

	#[derive(Debug)]
	pub(crate) struct EnigmaLine {
		line_number: usize,
		pub(crate) idents: usize,
		pub(crate) first_field: String,
		pub(crate) fields: Vec<String>,
	}

	impl EnigmaLine {
		pub(crate) fn new(line_number: usize, line: &str) -> Result<Option<EnigmaLine>> {
			let idents = line.chars().take_while(|x| *x == '\t').count();
			// TODO: there was some other code (related to inner classes?) that did this better!
			//  we may not use the count to index strings!, we must use the char_indices!
			let line = &line[idents..];

			// if the line is a `COMMENT` then it may contain `#`, otherwise everything after `#` is a comment
			let line = if line.starts_with(crate::enigma_file::COMMENT) {
				line
			} else if let Some((non_comment, _)) = line.split_once('#') {
				non_comment.trim()
			} else {
				line.trim()
			};

			if line.is_empty() {
				return Ok(None);
			}

			const JAVA_WHITESPACE: [char; 6] = [' ', '\t', '\n', '\x0b', '\x0c', '\x0d'];
			let mut fields = line.split(JAVA_WHITESPACE).map(|x| x.to_owned());

			let first_field = fields.next()
				.with_context(|| anyhow!("no first field in line {line_number}"))?;

			Ok(Some(EnigmaLine {
				line_number,
				idents,
				first_field,
				fields: fields.collect(),
			}))
		}
	}

	impl Line for EnigmaLine {
		fn get_idents(&self) -> usize {
			self.idents
		}
		fn get_line_number(&self) -> usize {
			self.line_number
		}
	}
}

fn write_class(class_key: &ClassNameSlice, class: &ClassNowodeMapping<2>, w: &mut impl Write, indent: usize) -> Result<()> {
	let indent = "\t".repeat(indent);

	let [_, dst] = class.info.names.names();
	// get to only the part after $ if it exists
	let src = class_key.get_inner_class_name().unwrap_or(class_key);
	// the dst name also stores only the inner class name
	let dst = dst.as_ref()
		.map(|dst| dst.get_inner_class_name().unwrap_or(dst));

	write!(w, "{indent}CLASS {src}")?;
	if let Some(dst) = dst {
		write!(w, " {dst}")?;
	}
	writeln!(w)?;

	if let Some(javadoc) = &class.javadoc {
		for line in javadoc.0.split('\n') {
			writeln!(w, "{indent}\tCOMMENT {line}")?;
		}
	}

	let mut fields: Vec<_> = class.fields.iter().collect();
	fields.sort_by(|a, b| a.1.info.names.cmp(&b.1.info.names).then_with(|| a.1.info.desc.cmp(&b.1.info.desc)));
	for (key, field) in fields {
		write!(w, "{indent}\tFIELD {}", key.name)?;
		let [_, dst] = field.info.names.names();
		if let Some(dst) = dst {
			write!(w, " {dst}")?;
		}
		writeln!(w, " {}", key.desc.as_inner())?;

		if let Some(javadoc) = &field.javadoc {
			for line in javadoc.0.split('\n') {
				writeln!(w, "{indent}\t\tCOMMENT {line}")?;
			}
		}
	}

	let mut methods: Vec<_> = class.methods.iter().collect();
	methods.sort_by(|a, b| a.1.info.names.cmp(&b.1.info.names).then_with(|| a.1.info.desc.cmp(&b.1.info.desc)));
	for (key, method) in methods {
		write!(w, "{indent}\tMETHOD {}", key.name)?;
		let [_, dst] = method.info.names.names();
		if let Some(dst) = dst.as_ref().filter(|&dst| dst != MethodName::INIT) {
			write!(w, " {dst}")?;
		}
		writeln!(w, " {}", key.desc.as_inner())?;

		if let Some(javadoc) = &method.javadoc {
			for line in javadoc.0.split('\n') {
				writeln!(w, "{indent}\t\tCOMMENT {line}")?;
			}
		}

		let mut parameters: Vec<_> = method.parameters.values().collect();
		parameters.sort_by_key(|x| &x.info);
		for parameter in parameters {
			let index = parameter.info.index;
			let [_, dst] = parameter.info.names.names();
			let dst = dst.as_ref().unwrap(); // TODO: unwrap

			writeln!(w, "{indent}\t\tARG {index} {}", dst.as_inner())?;

			if let Some(javadoc) = &parameter.javadoc {
				for line in javadoc.0.split('\n') {
					writeln!(w, "{indent}\t\t\tCOMMENT {line}")?;
				}
			}
		}
	}

	Ok(())
}

/// a node `( src, Node )` in the class-parent tree
#[derive(Clone, Copy)]
struct Node<'a> {
	src: &'a ClassNameSlice,
	class: &'a ClassNowodeMapping<2>,
}
struct Placement<'a> {
	/// `dst -> ( src, Node )` for all `Node`s without a parent
	///
	/// This is a `&str` because it's the file name, and not a "class name".
	file_map: IndexMap<&'a str, Node<'a>>,
	/// `parent src -> Vec<( child src, child Node )>` for all other nodes
	child_map: IndexMap<&'a ClassNameSlice, Vec<Node<'a>>>,
}

impl Placement<'_> {
	fn dbg(&self) {
		dbg!(self.file_map.keys().collect::<Vec<_>>());
		dbg!(self.child_map.iter().map(|(a, v)| (a, v.iter().map(|b| b.src).collect::<Vec<_>>())).collect::<IndexMap<_, _>>());
	}
}

/// Creates a mapping from path for file to a tree of class nodes to put in there
fn figure_out_files(mappings: &Mappings<2>) -> Placement<'_> {
	let mut child_map = IndexMap::new();
	let mut file_map = IndexMap::new();

	for (src, class) in &mappings.classes {
		// if the class has a parent that's in the mappings, don't create a file for it
		if let Some(parent) = src.get_inner_class_parent() {
			if mappings.classes.contains_key(parent) {
				// instead write it inside it's parent
				child_map.entry(parent).or_insert_with(Vec::new)
					.push(Node { src, class });

				continue;
			}
		}

		// in any other case, add a file to the output list
		let dst = class.info.names.names()[1].as_ref().map(|x| x.as_inner());
		let file_name = dst.unwrap_or(src.as_inner());
		file_map.insert(file_name, Node { src, class });
	}

	// maps can only contain one key each, not two equal keys
	file_map.sort_unstable_keys();
	child_map.sort_unstable_keys();

	// unstable sorting is fine, each .src only exists once
	child_map.values_mut()
		.for_each(|x| x.sort_unstable_by_key(|x| x.src));

	Placement { file_map, child_map }
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the complete mappings in the enigma format to the given writer.
///
/// Note that the enigma format usually splits the files based on the class names of the second namespace.
/// For writing them like that see TODO.
///
/// This method also adds in a small comment for each file that would've been generated
/// with the other write.
///
/// Note that this sorts the classes, fields, methods and parameters.
/// ```
/// # use pretty_assertions::assert_eq;
/// use quill::tree::mappings::Mappings;
/// let input = "\
/// CLASS D E
/// CLASS A B
/// 	FIELD bIsAfterA e I
/// 	FIELD bIsAfterAa d I
/// 	FIELD bIsAfterA e J
/// 	FIELD bIsAfterAa d J
/// 	METHOD methodA methodASecondName ()V
/// 	FIELD aIsBeforeB c I
/// 	CLASS C AnInnerClass
/// 		COMMENT A multiline
/// 		COMMENT comment.
/// 	METHOD methodB methodBSecondName ()
/// 	METHOD methodXa b (I)V
/// 	METHOD methodXb a (I)V
/// 	METHOD methodYa a (I)V
/// 	METHOD methodXa b (J)V
/// 	METHOD methodXb b (J)V
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mut mappings = Mappings::from_namespaces(["namespaceA", "namespaceB"]).unwrap();
/// quill::enigma_file::read_into(reader, &mut mappings).unwrap();
///
/// let mut vec = Vec::new();
/// quill::enigma_file::write_all(&mappings, &mut vec).unwrap();
/// let written = String::from_utf8(vec).unwrap();
///
/// let output = "#\n# B
/// CLASS A B
/// 	FIELD aIsBeforeB c I
/// 	FIELD bIsAfterA e I
/// 	FIELD bIsAfterA e J
/// 	FIELD bIsAfterAa d I
/// 	FIELD bIsAfterAa d J
/// 	METHOD methodA methodASecondName ()V
/// 	METHOD methodB methodBSecondName ()
/// 	METHOD methodXa b (I)V
/// 	METHOD methodXa b (J)V
/// 	METHOD methodXb a (I)V
/// 	METHOD methodXb b (J)V
/// 	METHOD methodYa a (I)V
/// 	CLASS C AnInnerClass
/// 		COMMENT A multiline
/// 		COMMENT comment.\n#\n# E
/// CLASS D E
/// ";
///
/// assert_eq!(written, output);
///
/// // the equivalent .tiny would look like this
///
/// let written = quill::tiny_v2::write_string(&mappings).unwrap();
///
/// let output = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	A	B
/// 	f	I	aIsBeforeB	c
/// 	f	I	bIsAfterA	e
/// 	f	I	bIsAfterAa	d
/// 	f	J	bIsAfterA	e
/// 	f	J	bIsAfterAa	d
/// 	m	()	methodB	methodBSecondName
/// 	m	()V	methodA	methodASecondName
/// 	m	(I)V	methodXa	b
/// 	m	(I)V	methodXb	a
/// 	m	(I)V	methodYa	a
/// 	m	(J)V	methodXa	b
/// 	m	(J)V	methodXb	b
/// c	A$C	B$AnInnerClass
/// 	c	A multiline\\ncomment.
/// c	D	E
/// ";
///
/// assert_eq!(written, output);
/// ```
pub fn write_all(mappings: &Mappings<2>, w: &mut impl Write) -> Result<()> {
	let f = figure_out_files(mappings);

	for (file_name, node) in f.file_map {
		writeln!(w, "#\n# {file_name}")?;
		write_one_tree_starting_at(node, &f.child_map, w)?;
	}

	Ok(())
}

pub(crate) fn write_all_for_each<W>(
	mappings: &Mappings<2>,
	mut make_writer: impl FnMut(&str) -> Result<W>,
) -> Result<()>
where
	W: Write,
{
	let f = figure_out_files(mappings);

	for (file_name, node) in f.file_map {
		let mut writer = make_writer(file_name)
			.with_context(|| anyhow!("failed to create writer for {file_name} (dst name)"))?;

		write_one_tree_starting_at(node, &f.child_map, &mut writer)
			.with_context(|| anyhow!("failed to write mappings to {file_name} (dst name)"))?;
	}

	Ok(())
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes one class in the enigma format to the given writer.
///
/// Note that "one class" includes all the inner classes of that class.
///
/// This is the single file version of the multi-file enigma format. The class name you supply is for the
/// second namespace, and is usually used to build the file names.
///
/// Note that this sorts the classes, fields, methods and parameters.
/// ```
/// # use pretty_assertions::assert_eq;
/// use quill::tree::mappings::Mappings;
/// let input = "\
/// CLASS D E
/// CLASS A B
/// 	FIELD bIsAfterA e I
/// 	FIELD bIsAfterAa d I
/// 	FIELD bIsAfterA e J
/// 	FIELD bIsAfterAa d J
/// 	METHOD methodA methodASecondName ()V
/// 	FIELD aIsBeforeB c I
/// 	CLASS C AnInnerClass
/// 		COMMENT A multiline
/// 		COMMENT comment.
/// 	METHOD methodB methodBSecondName ()
/// 	METHOD methodXa b (I)V
/// 	METHOD methodXb a (I)V
/// 	METHOD methodYa a (I)V
/// 	METHOD methodXa b (J)V
/// 	METHOD methodXb b (J)V
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mut mappings = Mappings::from_namespaces(["namespaceA", "namespaceB"]).unwrap();
/// quill::enigma_file::read_into(reader, &mut mappings).unwrap();
///
/// let mut vec = Vec::new();
/// quill::enigma_file::write_one(&mappings, "B", &mut vec).unwrap();
/// let written = String::from_utf8(vec).unwrap();
///
/// let output = "\
/// CLASS A B
/// 	FIELD aIsBeforeB c I
/// 	FIELD bIsAfterA e I
/// 	FIELD bIsAfterA e J
/// 	FIELD bIsAfterAa d I
/// 	FIELD bIsAfterAa d J
/// 	METHOD methodA methodASecondName ()V
/// 	METHOD methodB methodBSecondName ()
/// 	METHOD methodXa b (I)V
/// 	METHOD methodXa b (J)V
/// 	METHOD methodXb a (I)V
/// 	METHOD methodXb b (J)V
/// 	METHOD methodYa a (I)V
/// 	CLASS C AnInnerClass
/// 		COMMENT A multiline
/// 		COMMENT comment.
/// ";
///
/// assert_eq!(written, output);
/// ```
/// Note how the [`write_one`] call up there gets `"B"` and not `"A"`.
pub fn write_one(mappings: &Mappings<2>, dst_class_name: &str, w: &mut impl Write) -> Result<()> {
	let f = figure_out_files(mappings);

	let Some(&node) = f.file_map.get(dst_class_name) else {
		bail!("class {dst_class_name:?} (dst name) isn't parent-free");
	};

	write_one_tree_starting_at(node, &f.child_map, w)
}

fn write_one_tree_starting_at(
	node: Node,
	child_map: &IndexMap<&ClassNameSlice, Vec<Node>>,
	w: &mut impl Write
) -> Result<()> {
	let mut queue: VecDeque<_> = vec![ (node, 0) ].into();
	while let Some((parent, depth)) = queue.pop_front() {
		write_class(parent.src, parent.class, w, depth)?;

		if let Some(children) = child_map.get(parent.src) {
			for &child in children.iter().rev() {
				queue.push_front((child, depth + 1));
			}
		}
	}

	Ok(())
}
