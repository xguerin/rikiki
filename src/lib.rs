/*
 * FFI.
 */

extern "C" {
    /*
     * Slab allocation.
     */
    fn slab_new() -> *mut libc::c_void;
    fn slab_delete(slab: *mut libc::c_void) -> *mut libc::c_void;
    /*
     * Interpreter allocation.
     */
    fn lisp_new(slab: *mut libc::c_void) -> *mut libc::c_void;
    fn lisp_delete(lisp: *mut libc::c_void);
    /*
     * Modules.
     */
    fn module_load_defaults(lisp: *mut libc::c_void);
    /*
     * I/O.
     */
    fn lisp_io_push(lisp: *mut libc::c_void);
    fn lisp_io_pop(lisp: *mut libc::c_void);
    /*
     * Debug.
     */
    fn lisp_debug_parse_flags();
    /*
     * Modules.
     */
    fn module_init(lisp: *mut libc::c_void) -> bool;
    fn module_fini(lisp: *mut libc::c_void);
    /*
     * Atom.
     */
    fn lisp_make_nil(lisp: *mut libc::c_void) -> *const libc::c_void;
    fn lisp_make_true(lisp: *mut libc::c_void) -> *const libc::c_void;
    fn lisp_make_number(lisp: *mut libc::c_void, val: i64) -> *const libc::c_void;
    fn lisp_make_quote(lisp: *mut libc::c_void) -> *const libc::c_void;
    fn lisp_make_string(lisp: *mut libc::c_void, ptr: *const u8, len: usize)
        -> *const libc::c_void;
    /*
     * FFI-specific.
     */
    fn lisp_make_symbol_from_string(
        lisp: *mut libc::c_void,
        ptr: *const u8,
        len: usize,
    ) -> *const libc::c_void;
    fn lisp_get_type(atom: *const libc::c_void) -> i32;
    fn lisp_get_char(atom: *const libc::c_void) -> i8;
    fn lisp_get_number(atom: *const libc::c_void) -> i64;
    fn lisp_get_symbol(atom: *const libc::c_void) -> *const u8;
    fn lisp_drop(list: *mut libc::c_void, atom: *const libc::c_void);
    /*
     * Basic functions.
     */
    fn lisp_car(lisp: *mut libc::c_void, atom: *const libc::c_void) -> *const libc::c_void;
    fn lisp_cdr(lisp: *mut libc::c_void, atom: *const libc::c_void) -> *const libc::c_void;
    /*
     * List construction.
     */
    fn lisp_cons(
        lisp: *mut libc::c_void,
        a: *const libc::c_void,
        b: *const libc::c_void,
    ) -> *const libc::c_void;
    /*
     * Evaluation.
     */
    fn lisp_eval(
        lisp: *mut libc::c_void,
        clos: *const libc::c_void,
        atom: *const libc::c_void,
    ) -> *const libc::c_void;
    /*
     * Utilities.
     */
    fn lisp_load_file(lisp: *mut libc::c_void, path: *const u8) -> *const libc::c_void;
}

/*
 * Slab.
 */

pub struct Slab(*mut libc::c_void);

impl Default for Slab {
    fn default() -> Self {
        let inner = unsafe { slab_new() };
        Self(inner)
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe { slab_delete(self.0) };
    }
}

/*
 * Value.
 */

pub enum Value {
    None,
    Nil,
    True,
    Char(i8),
    Number(i64),
    Pair,
    Symbol(String),
    Wildcard,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Nil => write!(f, "Nil"),
            Value::True => write!(f, "T"),
            Value::Char(v) => write!(f, "{}", (*v as u8) as char),
            Value::Number(v) => write!(f, "{v}"),
            Value::Pair => write!(f, "Pair"),
            Value::Symbol(v) => write!(f, "{v}"),
            Value::Wildcard => write!(f, "_"),
        }
    }
}

/*
 * Atom.
 */

pub struct Atom(*mut libc::c_void, *const libc::c_void);

impl Atom {
    pub fn car(&self) -> Self {
        let inner = unsafe { lisp_car(self.0, self.1) };
        Self(self.0, inner)
    }

    pub fn cdr(&self) -> Self {
        let inner = unsafe { lisp_cdr(self.0, self.1) };
        Self(self.0, inner)
    }

    fn take(mut self) -> *const libc::c_void {
        let result = self.1;
        self.1 = std::ptr::null();
        result
    }

    pub fn value(&self) -> Value {
        let value = unsafe { lisp_get_type(self.1) };
        match value {
            0 => Value::None,
            1 => Value::Nil,
            2 => Value::True,
            3 => {
                let v = unsafe { lisp_get_char(self.1) };
                Value::Char(v)
            }
            4 => {
                let v = unsafe { lisp_get_number(self.1) };
                Value::Number(v)
            }
            5 => Value::Pair,
            6 => unsafe {
                let v = lisp_get_symbol(self.1);
                let w = &*v.cast::<[u8; 16]>();
                let i = w.iter().position(|&v| v == 0).unwrap_or(16);
                let r = String::from_utf8_lossy(&w[..i]);
                Value::Symbol(r.to_string())
            },
            7 => Value::Wildcard,
            _ => unreachable!(),
        }
    }
}

impl Drop for Atom {
    fn drop(&mut self) {
        if !self.1.is_null() {
            unsafe { lisp_drop(self.0, self.1) };
        }
    }
}

/*
 * Interpreter.
 */

pub struct Lisp(*mut libc::c_void);

impl Lisp {
    pub fn new(slab: &mut Slab) -> Option<Self> {
        unsafe {
            let inner = lisp_new(slab.0);
            if module_init(inner) {
                lisp_debug_parse_flags();
                module_load_defaults(inner);
                lisp_io_push(inner);
                Some(Self(inner))
            } else {
                lisp_delete(inner);
                None
            }
        }
    }

    pub fn load(&self, path: &str) -> Atom {
        let mut buffer: [u8; 256] = [0; 256];
        buffer[..path.len()].copy_from_slice(path.as_bytes());
        let result = unsafe { lisp_load_file(self.0, buffer.as_ptr()) };
        Atom(self.0, result)
    }

    pub fn nil(&self) -> Atom {
        let inner = unsafe { lisp_make_nil(self.0) };
        Atom(self.0, inner)
    }

    pub fn t(&self) -> Atom {
        let inner = unsafe { lisp_make_true(self.0) };
        Atom(self.0, inner)
    }

    pub fn number(&self, val: i64) -> Atom {
        let inner = unsafe { lisp_make_number(self.0, val) };
        Atom(self.0, inner)
    }

    pub fn quote(&self) -> Atom {
        let inner = unsafe { lisp_make_quote(self.0) };
        Atom(self.0, inner)
    }

    pub fn symbol(&self, v: &str) -> Atom {
        let inner = unsafe { lisp_make_symbol_from_string(self.0, v.as_ptr(), v.len()) };
        Atom(self.0, inner)
    }

    pub fn string(&self, v: &str) -> Atom {
        let inner = unsafe { lisp_make_string(self.0, v.as_ptr(), v.len()) };
        Atom(self.0, inner)
    }

    pub fn cons(&self, a: Atom, b: Atom) -> Atom {
        let value = unsafe { lisp_cons(self.0, a.take(), b.take()) };
        Atom(self.0, value)
    }

    pub fn eval(&self, clos: &Atom, atom: Atom) -> Atom {
        let result = unsafe { lisp_eval(self.0, clos.1, atom.take()) };
        Atom(self.0, result)
    }
}

impl Drop for Lisp {
    fn drop(&mut self) {
        unsafe {
            lisp_io_pop(self.0);
            module_fini(self.0);
            lisp_delete(self.0);
        }
    }
}
