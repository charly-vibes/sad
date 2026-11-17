//! Purpose: pin symbol-path chains.
//! Responsibilities: nest items inside impl and mod blocks.
//! Rationale: findings anchor to the nearest named ancestor chain.

// top-level comment outside any item

struct Foo;

impl Foo {
    // impl-level comment
    pub fn bar(&self) {
        // body comment
    }
}

mod x {
    // module-level comment

    pub fn nested() {
        // nested body comment
    }
}