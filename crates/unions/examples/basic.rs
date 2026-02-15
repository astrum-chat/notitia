//! A basic example of unions.

use std::any::type_name;

use unions::{IntoUnion, UnionPath, union};

struct Apple;
struct Pear;
struct Lemon;
struct Banana;

trait Fruit {
    fn name(&self) -> &'static str;
}

impl Fruit for Apple {
    fn name(&self) -> &'static str {
        "apple"
    }
}

impl Fruit for Pear {
    fn name(&self) -> &'static str {
        "pear"
    }
}

impl Fruit for Lemon {
    fn name(&self) -> &'static str {
        "lemon"
    }
}

impl Fruit for Banana {
    fn name(&self) -> &'static str {
        "fruit"
    }
}

// A helper macro for easily defining deeply nested unions.
type FruitUnion = union![Apple, Pear, Lemon, Banana];

fn main() {
    foo(Apple);
    foo(Pear);
    foo(Lemon);
    foo(Banana);
}

fn foo<P: UnionPath>(fruit: impl IntoUnion<FruitUnion, P> + Fruit) {
    println!("{}", fruit.name());
    println!("{} at {}", type_name_of_val(&fruit), type_name::<P>());
}

fn type_name_of_val<T>(_: &T) -> &'static str {
    type_name::<T>()
}
