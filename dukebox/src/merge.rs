use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{bail, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
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

fn visit_sided_annotation(class: ClassRepr, side: Side) -> Result<ClassRepr> {
	class.edit(|x| x.runtime_visible_annotations.push(sided_annotation(side)))
}


pub fn merge(client: impl Jar, server: impl Jar) -> Result<ParsedJar> {
	let mut opened_a = client.open()?;
	let mut opened_b = server.open()?;

	enum MergeCombination<C, S> {
		Client(C),
		Server(S),
		Both(C, S),
	}

	let keys = {
		enum MergeSide<C, S> {
			Client(C),
			Server(S),
		}

		let keys_a = opened_a.names().map(|x| (x.0, MergeSide::Client(x.1)));
		let keys_b = opened_b.names().map(|x| (x.0, MergeSide::Server(x.1)));
		let chain = keys_a.chain(keys_b);

		let (low, _) = chain.size_hint();
		let mut keys: IndexMap<String, MergeCombination<_, _>> = IndexMap::with_capacity(low / 2);

		for (key, client_or_server) in chain {
			match keys.entry(key.to_owned()) {
				Entry::Occupied(mut e) => {
					*e.get_mut() = match client_or_server {
						MergeSide::Client(c) => match e.get() {
							MergeCombination::Client(_) => unreachable!(),
							MergeCombination::Server(s) => MergeCombination::Both(c, *s),
							MergeCombination::Both(_, _) => unreachable!(),
						},
						MergeSide::Server(s) => match e.get() {
							MergeCombination::Client(c) => MergeCombination::Both(*c, s),
							MergeCombination::Server(_) => unreachable!(),
							MergeCombination::Both(_, _) => unreachable!(),
						},
					};
				},
				Entry::Vacant(e) => {
					e.insert(match client_or_server {
						MergeSide::Client(c) => MergeCombination::Client(c),
						MergeSide::Server(s) => MergeCombination::Server(s),
					});
				},
			}
		}

		keys.into_iter()
	};

	let mut resulting_entries = IndexMap::new();
	for (key, merge_combination) in keys {
		let result = match key.as_str() {
			"META-INF/MANIFEST.MF" => ParsedJarEntry::Other {
				attr: match merge_combination {
					MergeCombination::Client(c) => opened_a.by_entry_key(c)?.attrs(),
					MergeCombination::Server(s) => opened_b.by_entry_key(s)?.attrs(),
					MergeCombination::Both(c, _) => opened_a.by_entry_key(c)?.attrs(), // TODO: this ignores the server attrs...
				},
				data: b"Manifest-Version: 1.0\nMain-Class: net.minecraft.client.Main\n".to_vec(),
			},
			name if name.starts_with("META-INF/") && (name.ends_with(".SF") || name.ends_with(".RSA")) => {
				// remove these from the jar
				continue;
			},
			name => match merge_combination {
				MergeCombination::Client(c) => {
					let client = opened_a.by_entry_key(c)?.to_parsed_jar_entry()?;
					if let ParsedJarEntry::Class { attr, class } = client {
						ParsedJarEntry::Class { attr, class: visit_sided_annotation(class, Side::Client)? }
					} else {
						client
					}
				},
				MergeCombination::Server(s) => {
					// skip the libraries the server bundles
					if name.ends_with(".class") && !name.starts_with("net/minecraft/") && name.contains('/') {
						continue;
					}

					let server = opened_b.by_entry_key(s)?.to_parsed_jar_entry()?;
					if let ParsedJarEntry::Class { attr, class } = server {
						ParsedJarEntry::Class { attr, class: visit_sided_annotation(class, Side::Server)? }
					} else {
						server
					}
				},
				MergeCombination::Both(c, s) => {
					let client = opened_a.by_entry_key(c)?.to_parsed_jar_entry()?;
					let server = opened_b.by_entry_key(s)?.to_parsed_jar_entry()?;
					match (client, server) {
						(
							ParsedJarEntry::Class { attr: client_attr, class: client_ },
							ParsedJarEntry::Class { attr: server_attr, class: server_ }
						) => {

							let c_vec = client_.write_from_ref()?;
							let s_vec = server_.write_from_ref()?;

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
								eprintln!("warn: merging {name:?} from both client and server not implemented, taking client version");
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
			},
		};

		resulting_entries.insert(key, result);
	}

	Ok(ParsedJar { entries: resulting_entries })
}