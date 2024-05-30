use std::fmt::Debug;
use std::hash::Hash;
use std::io::{Cursor, Read, Seek, Write};
use anyhow::{bail, Result};
use indexmap::IndexMap;
use zip::{DateTime, ZipArchive, ZipWriter};
use zip::read::ZipFile;
use zip::write::FileOptions;
use duke::tree::annotation::{Annotation, ElementValue, ElementValuePair};
use duke::tree::class::{ClassFile, ClassName};
use duke::tree::field::{Field, FieldDescriptor};
use duke::tree::method::Method;
use crate::jar::{Jar, MemJar};

#[derive(Clone, Debug, PartialEq)]
enum Side {
	Both,
	Client,
	Server,
}

#[derive(Clone, Debug)]
struct Entry {
	kind: EntryKind,
	/// path, without stripped `/` at the start
	path: String,
	attr: BasicFileAttributes,
	data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
enum EntryKind {
	Class,
	Other,
}

#[derive(Clone, Debug)]
struct BasicFileAttributes {
	mtime: DateTime,
	atime: (),
	ctime: (),
}

impl BasicFileAttributes {
	fn new(file: &ZipFile) -> BasicFileAttributes {
		// TODO: implement reading the more exact file modification times from the extra data of the zip file
		BasicFileAttributes {
			mtime: file.last_modified(),
			atime: (),
			ctime: (),
		}
	}
}

fn merge_preserve_order<'a, T: Clone + PartialEq>(a: &'a [T], b: &'a [T]) -> std::vec::IntoIter<&'a T> {
	let mut ai = a.iter().peekable();
	let mut bi = b.iter().peekable();

	let mut r = Vec::new();

	while ai.peek().is_some() || bi.peek().is_some() {
		let mut no_change = true;

		while let Some(x) = ai.next_if(|x| bi.peek().is_some_and(|b| b == x)) {
			r.push(x);
			no_change = false;
		}

		while let Some(x) = ai.next_if(|x| !b.contains(x)) {
			r.push(x);
			no_change = false;
		}
		while let Some(x) = bi.next_if(|x| !b.contains(x)) {
			r.push(x);
			no_change = false;
		}

		// if the order is scrambled, it's not possible to merge
		// the lists while preserving the order from both sides
		if no_change {
			break;
		}
	}

	r.extend(ai);
	r.extend(bi.filter(|b_i| !a.contains(b_i)));

	r.into_iter()
}

fn merge_slice<T, U, N, S, I>(client: &[T], server: &[T], name: N, side: S, inner: I) -> Result<Vec<T>>
	where
		T: Clone + PartialEq,
		U: Clone + PartialEq + Eq + Hash,
		N: Copy + Fn(&T) -> U,
		S: Fn(&T, Side) -> Result<T>,
		I: Fn(&T, &T) -> Result<T>,
{
	let lc: Vec<_> = client.iter().map(name).collect();
	let ls: Vec<_> = server.iter().map(name).collect();

	let c: IndexMap<_, _> = client.iter().map(|i| (name(i), i)).collect();
	let s: IndexMap<_, _> = server.iter().map(|i| (name(i), i)).collect();

	merge_preserve_order(&lc, &ls)
		.map(|i| match (c.get(i), s.get(i)) {
			(Some(&ec), Some(&es)) if ec == es => Ok(ec.clone()),
			(Some(&ec), Some(&es)) => inner(ec, es),
			(Some(&ec), None) => side(ec, Side::Client),
			(None, Some(&es)) => side(es, Side::Server),
			(None, None) => unreachable!(),
		})
		.collect()
}

fn merge_eq<T>(client: &T, server: &T) -> Result<T>
	where
		T: Debug + Clone + PartialEq,
{
	if client != server {
		bail!("cannot merge not equal client {client:?} and server {server:?}");
	}
	Ok(client.clone())
}

fn merge_from_client<T>(client: &T, server: &T) -> Result<T>
	where
		T: Debug + Clone + PartialEq,
{
	pretty_assertions::assert_eq!(client, server);
	Ok(client.clone())
}


fn sided_annotation(side: Side) -> Annotation {
	Annotation {
		annotation_type: FieldDescriptor::from("Lnet/fabricmc/api/Environment;"),
		element_value_pairs: vec![
			ElementValuePair {
				name: "value".to_owned(),
				value: ElementValue::Enum {
					type_name: FieldDescriptor::from("Lnet/fabricmc/api/EnvType;"),
					const_name: match side {
						Side::Both => unreachable!(),
						Side::Client => "CLIENT".to_owned(),
						Side::Server => "SERVER".to_owned(),
					},
				}
			}
		],
	}
}

fn class_merger_merge(client: &[u8], server: &[u8]) -> Result<Vec<u8>> {
	let client = duke::read_class(&mut Cursor::new(client))?;
	let server = duke::read_class(&mut Cursor::new(server))?;

	let interfaces: Vec<_> = merge_preserve_order(&client.interfaces, &server.interfaces).collect();

	let mut ci = Vec::new();
	let mut si = Vec::new();
	for i in &interfaces {
		let nc = client.interfaces.contains(i);
		let ns = server.interfaces.contains(i);
		if nc && !ns {
			ci.push(*i);
		} else if ns && !nc {
			si.push(*i);
		}
	}

	let out = ClassFile {
		version: merge_from_client(&client.version, &server.version)?,
		access: merge_from_client(&client.access, &server.access)?,
		name: merge_eq(&client.name, &server.name)?,
		super_class: merge_eq(&client.super_class, &server.super_class)?,
		interfaces: interfaces.into_iter().cloned().collect(),

		fields: merge_slice(
			&client.fields,
			&server.fields,
			|field| (field.name.clone(), field.descriptor.clone()),
			|field, side| {
				let mut field = field.clone();
				field.runtime_invisible_annotations.push(sided_annotation(side));
				Ok(field)
			},
			|client, server| Ok(Field {
				//TODO: figure out a way to merge both flags,
				// make use of pretty_assertions::assert_eq!(client.access, server.access); to see what differs
				access: client.access,
				name: merge_eq(&client.name, &server.name)?,
				descriptor: merge_eq(&client.descriptor, &server.descriptor)?,

				has_deprecated_attribute: merge_from_client(&client.has_deprecated_attribute, &server.has_deprecated_attribute)?,
				has_synthetic_attribute: merge_from_client(&client.has_synthetic_attribute, &server.has_synthetic_attribute)?,

				..client.clone() // TODO: handle more fields
			})
		)?,
		methods: merge_slice(
			&client.methods,
			&server.methods,
			|method| (method.name.clone(), method.descriptor.clone()),
			|method, side| {
				let mut method = method.clone();
				method.runtime_invisible_annotations.push(sided_annotation(side));
				Ok(method)
			},
			|client, server| Ok(Method {
				//TODO: figure out a way to merge both flags,
				// make use of pretty_assertions::assert_eq!(client.access, server.access); to see what differs
				access: client.access,
				name: merge_eq(&client.name, &server.name)?,
				descriptor: merge_eq(&client.descriptor, &server.descriptor)?,

				has_deprecated_attribute: merge_from_client(&client.has_deprecated_attribute, &server.has_deprecated_attribute)?,
				has_synthetic_attribute: merge_from_client(&client.has_synthetic_attribute, &server.has_synthetic_attribute)?,

				..client.clone() // TODO: handle more fields
			}),
		)?,

		has_deprecated_attribute: merge_from_client(&client.has_deprecated_attribute, &server.has_deprecated_attribute)?,
		has_synthetic_attribute: merge_from_client(&client.has_synthetic_attribute, &server.has_synthetic_attribute)?,

		inner_classes: {
			let inner_classes = merge_slice(
				&client.inner_classes.unwrap_or_default(),
				&server.inner_classes.unwrap_or_default(),
				|inner_class| inner_class.inner_class.clone(),
				|inner_class, _| Ok(inner_class.clone()),
				|client, server| {
					pretty_assertions::assert_eq!(client, server);
					panic!();
				}
			)?;
			if inner_classes.is_empty() {
				None
			} else {
				Some(inner_classes)
			}
		},
		enclosing_method: client.enclosing_method,
		signature: client.signature,

		source_file: client.source_file,
		source_debug_extension: client.source_debug_extension,

		runtime_visible_annotations: client.runtime_visible_annotations,
		runtime_invisible_annotations: {

			let mut x = client.runtime_invisible_annotations;

			fn make_annotation(i: &ClassName, side: Side) -> ElementValue {
				ElementValue::AnnotationInterface(Annotation {
					annotation_type: FieldDescriptor::from("Lnet/fabricmc/api/EnvironmentInterface;"),
					element_value_pairs: vec![
						ElementValuePair {
							name: "value".to_owned(),
							value: ElementValue::Enum {
								type_name: FieldDescriptor::from("Lnet/fabricmc/api/EnvType;"),
								const_name: match side {
									Side::Both => unreachable!(),
									Side::Client => "CLIENT".to_owned(),
									Side::Server => "SERVER".to_owned(),
								},
							},
						},
						ElementValuePair {
							name: "itf".to_owned(),
							value: ElementValue::Class({
								let mut s = String::new();
								s.push('L');
								s.push_str(i.as_str());
								s.push(';');
								s
							})
						}
					],
				})
			}

			let c = ci.into_iter().map(|i| make_annotation(i, Side::Client));
			let s = si.into_iter().map(|i| make_annotation(i, Side::Server));

			let array: Vec<_> = c.chain(s).collect();

			if !array.is_empty() {
				let annotation = Annotation {
					annotation_type: FieldDescriptor::from("Lnet/fabricmc/api/EnvironmentInterfaces;"),
					element_value_pairs: vec![
						ElementValuePair {
							name: "value".to_owned(),
							value: ElementValue::ArrayType(array),
						}
					],
				};

				x.push(annotation);
			}

			x
		},
		runtime_visible_type_annotations: client.runtime_visible_type_annotations,
		runtime_invisible_type_annotations: client.runtime_invisible_type_annotations,

		module: client.module,
		module_packages: client.module_packages,
		module_main_class: client.module_main_class,

		nest_host_class: client.nest_host_class,
		nest_members: client.nest_members,
		permitted_subclasses: None, // TODO: deal with this here

		record_components: vec![], // TODO: deal with this here

		attributes: client.attributes,
	};

	let mut buf = Vec::new();
	duke::write_class(&mut buf, &out)?;
	Ok(buf)
}

fn sided_class_visitor(data: &[u8], side: Side) -> Result<Vec<u8>> {
	let mut class = duke::read_class(&mut Cursor::new(data))?;

	class.runtime_visible_annotations.push(sided_annotation(side));

	let mut buf = Vec::new();
	duke::write_class(&mut buf, &class)?;
	Ok(buf)
}


fn read_to_map(jar: &impl Jar) -> Result<IndexMap<String, Entry>> {
	fn action(mut reader: impl Read + Seek) -> Result<IndexMap<String, Entry>> {
		let mut zip = ZipArchive::new(&mut reader)?;

		let mut map = IndexMap::new();

		for index in 0..zip.len() {
			let mut file = zip.by_index(index)?;

			if file.is_dir() {
				continue;
			}

			match file.name().to_owned().as_str() {
				name if name.ends_with(".class") => {
					let mut vec = Vec::new();
					file.read_to_end(&mut vec)?;

					let e = Entry {
						kind: EntryKind::Class,
						path: name.to_owned(),
						attr: BasicFileAttributes::new(&file),
						data: vec,
					};
					map.insert(e.path.clone(), e);
				},
				name @ "/META-INF/MANIFEST.MF" => {
					let v = b"Manifest-Version: 1.0\nMain-Class: net.minecraft.client.Main\n".to_vec();

					let e = Entry {
						kind: EntryKind::Other,
						path: name.to_owned(),
						attr: BasicFileAttributes::new(&file),
						data: v,
					};
					map.insert(e.path.clone(), e);
				},
				name if name.starts_with("/META-INF/") && (
					name.ends_with(".SF") || name.ends_with(".RSA")) => {
				},
				name => {
					let mut vec = Vec::new();
					file.read_to_end(&mut vec)?;

					let e = Entry {
						kind: EntryKind::Other,
						path: name.to_owned(),
						attr: BasicFileAttributes::new(&file),
						data: vec,
					};
					map.insert(e.path.clone(), e);
				},
			}
		}

		Ok(map)
	}

	let reader = jar.open()?;
	action(reader)
}

pub(crate) fn merge(client: impl Jar, server: impl Jar) -> Result<MemJar> {
	let entries_client = read_to_map(&client)?;
	let entries_server = read_to_map(&server)?;

	let mut resulting_entries = Vec::new();
	for key in entries_client.keys().chain(entries_server.keys()) {
		let is_class = key.ends_with(".class");

		let entry_client = entries_client.get(key);
		let entry_server = entries_server.get(key);

		let side = match (entry_client, entry_server) {
			(Some(_), Some(_)) => Side::Both,
			(Some(_), None) => Side::Client,
			(None, Some(_)) => Side::Server,
			(None, None) => unreachable!(),
		};

		let is_minecraft = entries_client.contains_key(key)
			|| key.starts_with("/net/minecraft/")
			|| !key.strip_prefix('/').is_some_and(|x| x.contains('/'));

		if !(is_class && !is_minecraft && side == Side::Server) {
			let result = match (entry_client, entry_server) {
				(Some(client), Some(server)) if client.data == server.data => client.clone(),
				(Some(client), Some(server)) => {
					if is_class {
						assert_eq!(&client.kind, &server.kind);
						assert_eq!(&client.path, &server.path);

						Entry {
							kind: client.kind.clone(),
							path: client.path.clone(),
							attr: client.attr.clone(), // TODO: ?= server.attr
							data: class_merger_merge(&client.data, &server.data)?,
						}
					} else {
						// TODO: warning here
						client.clone()
					}
				},
				(Some(client), None) => client.clone(),
				(None, Some(server)) => server.clone(),
				(None, None) => unreachable!(),
			};

			let r = if is_class && is_minecraft && side != Side::Both {
				Entry {
					kind: EntryKind::Class,
					path: result.path,
					attr: result.attr,
					data: sided_class_visitor(&result.data, side)?,
				}
			} else {
				result
			};

			resulting_entries.push(r);
		}
	}

	let writer = Cursor::new(Vec::new());
	let mut zip_out = ZipWriter::new(writer);

	for e in resulting_entries {

		let mut x = e.path.as_str();
		while let Some((left, _)) = x.rsplit_once('/') {
			if !left.is_empty() {
				zip_out.add_directory(left, FileOptions::default())?;
			}
			x = left;
		}

		zip_out.start_file(e.path, FileOptions::default().last_modified_time(e.attr.mtime))?;
		// TODO: set the files ctime, atime, mtime to the ones from the file read
		zip_out.write_all(&e.data)?;

	}

	let vec = zip_out.finish()?.into_inner();

	Ok(MemJar::new_unnamed(vec))
}