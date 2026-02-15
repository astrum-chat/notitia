//! An example of constructing unions via a builder pattern.

use std::marker::PhantomData;
use unions::{IntoUnion, Union, UnionPath};

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
        "pumpkin"
    }
}

struct Basket<T> {
    _marker: PhantomData<T>,
    fruit: Vec<Box<dyn Fruit>>,
}

impl Basket<()> {
    #[allow(unused)]
    fn new<T: Fruit + 'static>() -> Basket<T> {
        Basket {
            _marker: PhantomData,
            fruit: vec![],
        }
    }
}

impl<T> Basket<T> {
    #[allow(unused)]
    fn allow<U: Fruit>(self) -> Basket<Union<T, U>> {
        Basket {
            _marker: PhantomData,
            fruit: self.fruit,
        }
    }

    fn push<P: UnionPath>(&mut self, fruit: impl IntoUnion<T, P> + Fruit + 'static) {
        self.fruit.push(Box::new(fruit));
    }
}

fn main() {
    let basket = Basket::new::<Apple>();
    // type: Basket<Apple>

    let basket = basket.allow::<Pear>();
    // type: Basket<Union<Apple, Pear>>

    #[allow(unused)]
    let mut basket = basket.allow::<Lemon>();
    // type: Basket<Union<Union<Apple, Pear>, Lemon>>

    // This fails as Banana is not allowed in the basket.
    //basket.push(Banana);

    let mut basket = basket.allow::<Banana>();
    // type: Basket<Union<Union<Union<Apple, Pear>, Lemon>, Banana>>

    // Banana is now allowed in the basket.
    basket.push(Banana);

    basket.push(Apple);
    basket.push(Lemon);

    for fruit in basket.fruit {
        println!("{}", fruit.name());
    }
}
