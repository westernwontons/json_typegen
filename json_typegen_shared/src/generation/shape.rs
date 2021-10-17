use linked_hash_map::LinkedHashMap;

use crate::generation::value::{pretty_print_value, Value};
use crate::options::Options;
use crate::shape::{self, Shape};
use crate::to_singular::to_singular;
use crate::util::string_hashmap;

#[allow(dead_code)]
pub struct Ctxt {
    options: Options,
}

pub type Code = String;

pub fn shape_string(name: &str, shape: &Shape, options: Options) -> Code {
    let mut ctxt = Ctxt { options };

    let value = type_from_shape(&mut ctxt, name, shape);

    pretty_print_value(0, &value)
}

fn type_from_shape(ctxt: &mut Ctxt, path: &str, shape: &Shape) -> Value {
    use crate::shape::Shape::*;
    match *shape {
        Null => Value::Null,
        Any => Value::Str("any"),
        Bottom => Value::Str("bottom"),
        Bool => Value::Str("bool"),
        StringT => Value::Str("string"),
        Integer => Value::Str("integer"),
        Floating => Value::Str("floating"),
        Tuple(ref shapes, _n) => {
            let folded = shape::fold_shapes(shapes.clone());
            if folded == Any && shapes.iter().any(|s| s != &Any) {
                generate_tuple_type(ctxt, path, shapes)
            } else {
                generate_vec_type(ctxt, path, &folded)
            }
        }
        VecT { elem_type: ref e } => generate_vec_type(ctxt, path, e),
        Struct { fields: ref map } => generate_struct_from_field_shapes(ctxt, path, map),
        MapT { val_type: ref v } => generate_map_type(ctxt, path, v),
        Opaque(ref t) => Value::String(t.to_string()),
        Optional(ref e) => Value::Object(string_hashmap! {
            "__type__" => Value::Str("optional"),
            "item" => type_from_shape(ctxt, path, e),
        }),
    }
}

fn generate_vec_type(ctxt: &mut Ctxt, path: &str, shape: &Shape) -> Value {
    let singular = to_singular(path);
    let inner = type_from_shape(ctxt, &singular, shape);
    Value::Array(vec![inner])
}

fn generate_map_type(ctxt: &mut Ctxt, path: &str, shape: &Shape) -> Value {
    let singular = to_singular(path);
    let inner = type_from_shape(ctxt, &singular, shape);
    Value::Object(string_hashmap! {
        "__type__" => Value::Str("map"),
        "values" => inner
    })
}

fn generate_tuple_type(ctxt: &mut Ctxt, path: &str, shapes: &[Shape]) -> Value {
    let mut types = Vec::new();

    for shape in shapes {
        let typ = type_from_shape(ctxt, path, shape);
        types.push(typ);
    }

    Value::Object(string_hashmap! {
        "__type__" => Value::Str("tuple"),
        "items" => Value::Array(types),
    })
}

fn collapse_option(typ: &Shape) -> (bool, &Shape) {
    if let Shape::Optional(inner) = typ {
        return (true, &**inner);
    }
    (false, typ)
}

fn generate_struct_from_field_shapes(
    ctxt: &mut Ctxt,
    _path: &str,
    map: &LinkedHashMap<String, Shape>,
) -> Value {
    let mut properties = LinkedHashMap::new();

    for (name, typ) in map.iter() {
        let (was_optional, collapsed) = collapse_option(typ);

        let annotated_name = if was_optional {
            name.to_owned() + "?"
        } else {
            name.to_owned()
        };

        let field_code = type_from_shape(ctxt, name, collapsed);

        properties.insert(annotated_name, field_code);
    }

    Value::Object(properties)
}
