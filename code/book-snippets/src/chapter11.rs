//! Rust counterparts for the OCaml datatype fragments in Chapter 11.

pub mod schematic_variant {
    // TAPL-SNIPPET-BEGIN: ch11-variant-schematic
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Variant<T1, T2> {
        L1(T1),
        L2(T2),
    }
    // TAPL-SNIPPET-END: ch11-variant-schematic
}

pub mod concrete_variant {
    // TAPL-SNIPPET-BEGIN: ch11-variant-concrete
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum T<T1, TN> {
        L1(T1),
        Ln(TN),
    }
    // TAPL-SNIPPET-END: ch11-variant-concrete
}

// TAPL-SNIPPET-BEGIN: ch11-weekday
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
}
// TAPL-SNIPPET-END: ch11-weekday

pub type Nat = u64;

// TAPL-SNIPPET-BEGIN: ch11-nat-list
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NatList {
    Nil,
    Cons(Nat, Box<NatList>),
}
// TAPL-SNIPPET-END: ch11-nat-list

// TAPL-SNIPPET-BEGIN: ch11-generic-list
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum List<T> {
    Nil,
    Cons(T, Box<List<T>>),
}
// TAPL-SNIPPET-END: ch11-generic-list

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_lists_construct_values() {
        let naturals = NatList::Cons(1, Box::new(NatList::Nil));
        assert_eq!(naturals, NatList::Cons(1, Box::new(NatList::Nil)));

        let words = List::Cons("tapl", Box::new(List::Nil));
        assert_eq!(words, List::Cons("tapl", Box::new(List::Nil)));
    }
}
