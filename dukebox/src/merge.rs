use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use duke::tree::annotation::{Annotation, ElementValue, ElementValuePair};
use duke::tree::class::{ClassFile, ClassName};
use duke::tree::field::{Field, FieldDescriptor};
use duke::tree::method::Method;
use crate::{Jar, JarEntry, OpenedJar};
use crate::lazy_duke::ClassRepr;
use crate::parsed::{ParsedJar, ParsedJarEntry};

#[derive(Clone, Debug, PartialEq)]
enum Side {
	Client,
	Server,
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
						Side::Client => "CLIENT".to_owned(),
						Side::Server => "SERVER".to_owned(),
					},
				}
			}
		],
	}
}

fn class_merger_merge(client: ClassRepr, server: ClassRepr) -> Result<ClassRepr> {
	let client = client.read()?;
	let server = server.read()?;

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

	Ok(ClassFile {
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
	}.into())
}

fn visit_sided_annotation(side: Side) -> impl FnOnce(&mut ClassFile) {
	|x| x.runtime_visible_annotations.push(sided_annotation(side))
}

pub fn merge(client: impl Jar, server: impl Jar) -> Result<ParsedJar> {
	let mut client = client.open()?;
	let mut server = server.open()?;

	let keys_client = client.names().map(|x| x.as_ref().to_owned());
	let keys_server = server.names().map(|x| x.as_ref().to_owned());

	let keys: IndexSet<_> = keys_client.chain(keys_server).collect();

	let mut resulting_entries = ParsedJar { entries: IndexMap::new(), };
	for key in keys {

		let entry_client = client.by_name(&key)?;
		let entry_server = server.by_name(&key)?;

		let is_minecraft = entry_client.is_some()
			|| key.starts_with("/net/minecraft/")
			|| !key.strip_prefix('/').is_some_and(|x| x.contains('/'));

		fn do_other(key: &str, entry: ParsedJarEntry) -> Option<ParsedJarEntry> {
			if let ParsedJarEntry::Other { attr, data } = entry {

				match key {
					"/META-INF/MANIFEST.MF" => Some(ParsedJarEntry::Other {
						attr,
						data: b"Manifest-Version: 1.0\nMain-Class: net.minecraft.client.Main\n".to_vec(),
					}),
					name if name.starts_with("/META-INF/") && (name.ends_with(".SF") || name.ends_with(".RSA")) => None,
					_ => Some(ParsedJarEntry::Other { attr, data }),
				}

			} else {
				Some(entry)
			}
		}


		let entry_client = entry_client.map(|x| x.to_parsed_jar_entry()).transpose()?;
		let entry_server = entry_server.map(|x| x.to_parsed_jar_entry()).transpose()?;

		let entry_client = entry_client.and_then(|x| do_other(&key, x));
		let entry_server = entry_server.and_then(|x| do_other(&key, x));


		let is_server = entry_client.is_none() && entry_server.is_some();

		let is_class = key.ends_with(".class");

		if is_class && !is_minecraft && is_server {
			continue;
		}

		let result = match (entry_client, entry_server) {
			(Some(client_), Some(server_)) => {

				match (client_, server_) {
					(
						ParsedJarEntry::Class { attr: client_attr, class: client_ },
						ParsedJarEntry::Class { attr: server_attr, class: server_ }
					) => {

						let c_vec = client.by_name(&key)?.unwrap().to_vec()?;
						let s_vec = server.by_name(&key)?.unwrap().to_vec()?;

						if c_vec == s_vec {
							ParsedJarEntry::Class { attr: client_attr, class: client_ }
						} else {
							ParsedJarEntry::Class {
								attr: client_attr, // TODO: ?= server_attr
								class: class_merger_merge(client_, server_)?,
							}
						}
					},
					(
						ParsedJarEntry::Other { attr: client_attr, data: client },
						ParsedJarEntry::Other { attr: server_attr, data: server }
					) => {
						if client == server {
							ParsedJarEntry::Other { attr: client_attr, data: client }
						} else {
							// TODO: warning here
							ParsedJarEntry::Other { attr: client_attr, data: client }
						}
					},
					(
						ParsedJarEntry::Dir { attr: client_attr },
						ParsedJarEntry::Dir { attr: server_attr  },
					) => {
						// TODO: check _attr in all of these!
						ParsedJarEntry::Dir { attr: client_attr }
					},
					(c, s) => {
						bail!("types don't match {c:?} and {s:?}")
					},
				}
			},
			(Some(client), None) => {
				if let ParsedJarEntry::Class { attr, class } = client {
					ParsedJarEntry::Class {
						attr,
						class: if is_minecraft {
							class.edit(visit_sided_annotation(Side::Client))?
						} else {
							class
						},
					}
				} else {
					client
				}
			},
			(None, Some(server)) => {
				if let ParsedJarEntry::Class { attr, class } = server {
					ParsedJarEntry::Class {
						attr,
						class: if is_minecraft {
							class.edit(visit_sided_annotation(Side::Server))?
						} else {
							class
						},
					}
				} else {
					server
				}
			},
			(None, None) => continue,
		};

		resulting_entries.put(key.clone(), result)?;
	}

	Ok(resulting_entries)
}