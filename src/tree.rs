

#[derive(Debug, Clone)]
pub(crate) struct Mapping<I, C, F, M, P> {
	info: I,
	classes: Vec<Class<C, F, M, P>>
}

impl<I, C, F, M, P> Mapping<I, C, F, M, P> {
	pub(crate) fn new(info: I) -> Mapping<I, C, F, M, P> {
		Mapping {
			info,
			classes: Vec::new(),
		}
	}

	pub(crate) fn add_class(&mut self, class: Class<C, F, M, P>) {
		self.classes.push(class);
	}

	pub(crate) fn classes(&self) -> impl Iterator<Item=&Class<C, F, M, P>> {
		self.classes.iter()
	}
}

#[derive(Debug, Clone)]
pub(crate) struct Class<C, F, M, P> {
	info: C,
	fields: Vec<Field<F>>,
	methods: Vec<Method<M, P>>,
}

impl<C, F, M, P> Class<C, F, M, P> {
	pub(crate) fn new(class: C) -> Class<C, F, M, P> {
		Class {
			info: class,
			fields: Vec::new(),
			methods: Vec::new(),
		}
	}

	pub(crate) fn add_field(&mut self, field: Field<F>) {
		self.fields.push(field);
	}

	pub(crate) fn add_method(&mut self, method: Method<M, P>) {
		self.methods.push(method);
	}

	pub(crate) fn fields(&self) -> impl Iterator<Item=&Field<F>> {
		self.fields.iter()
	}

	pub(crate) fn methods(&self) -> impl Iterator<Item=&Method<M, P>> {
		self.methods.iter()
	}

	pub(crate) fn inner(&self) -> &C {
		&self.info
	}

	pub(crate) fn inner_mut(&mut self) -> &mut C {
		&mut self.info
	}
}

#[derive(Debug, Clone)]
pub(crate) struct Field<F> {
	info: F,
}

impl<F> Field<F> {
	pub(crate) fn new(field: F) -> Field<F> {
		Field {
			info: field,
		}
	}

	pub(crate) fn inner(&self) -> &F {
		&self.info
	}

	pub(crate) fn inner_mut(&mut self) -> &mut F {
		&mut self.info
	}
}

#[derive(Debug, Clone)]
pub(crate) struct Method<M, P> {
	info: M,
	parameters: Vec<Parameter<P>>
}

impl<M, P> Method<M, P> {
	pub(crate) fn new(method: M) -> Method<M, P> {
		Method {
			info: method,
			parameters: Vec::new(),
		}
	}

	pub(crate) fn add_parameter(&mut self, parameter: Parameter<P>) {
		self.parameters.push(parameter);
	}

	pub(crate) fn parameters(&self) -> impl Iterator<Item=&Parameter<P>> {
		self.parameters.iter()
	}

	pub(crate) fn inner(&self) -> &M {
		&self.info
	}

	pub(crate) fn inner_mut(&mut self) -> &mut M {
		&mut self.info
	}
}

#[derive(Debug, Clone)]
pub(crate) struct Parameter<P> {
	info: P,
}

impl<P> Parameter<P> {
	pub(crate) fn new(parameter: P) -> Parameter<P> {
		Parameter {
			info: parameter,
		}
	}

	pub(crate) fn inner(&self) -> &P {
		&self.info
	}

	pub(crate) fn inner_mut(&mut self) -> &mut P {
		&mut self.info
	}
}