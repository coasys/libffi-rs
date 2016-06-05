//! Representations of C types and arrays thereof.

use std::fmt;
use std::mem;
use std::ptr::{Unique, self};
use libc;

use low;

// Internally we represent types and type arrays using raw pointers,
// since this is what libffi understands. Below we wrap them with
// types that implement Drop and Clone.

type Type_      = *mut low::ffi_type;
type TypeArray_ = *mut Type_;

// Informal indication that the object should be considered owned by
// the given reference.
type Owned<T>      = T;

/// Represents a single C type.
pub struct Type(Unique<low::ffi_type>);

/// Represents a sequence of C types, which can be used to construct
/// a struct type or as the arguments when creating a
/// [CIF](../middle/struct.Cif.html).
pub struct TypeArray(Unique<*mut low::ffi_type>);

impl fmt::Debug for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("Type({:?})", *self.0))
    }
}

impl fmt::Debug for TypeArray {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("TypeArray({:?})", *self.0))
    }
}


/// Computes the length of a raw `TypeArray_` by searching for the
/// null terminator.
unsafe fn ffi_type_array_len(mut array: TypeArray_) -> usize {
    let mut count   = 0;
    while !(*array).is_null() {
        count += 1;
        array = array.offset(1);
    }
    count
}

/// Creates an empty `TypeArray_` with null terminator.
unsafe fn ffi_type_array_create_empty(len: usize) -> Owned<TypeArray_> {
    let array = libc::malloc((len + 1) * mem::size_of::<Type_>())
                    as TypeArray_;
    assert!(!array.is_null());
    *array.offset(len as isize) = ptr::null::<low::ffi_type>() as Type_;
    array
}

/// Creates a null-terminated array of Type_. Takes ownership of
/// the elements.
unsafe fn ffi_type_array_create(elements: Vec<Type>)
    -> Owned<TypeArray_>
{
    let size = elements.len();
    let new  = ffi_type_array_create_empty(size);
    for i in 0 .. size {
        *new.offset(i as isize) = *elements[i].0;
    }

    for t in elements {
        mem::forget(t);
    }

    new
}

/// Creates a struct type from a raw array of element types.
unsafe fn ffi_type_struct_create_raw(elements: Owned<TypeArray_>)
    -> Owned<Type_>
{
    let new = libc::malloc(mem::size_of::<low::ffi_type>()) as Type_;
    assert!(!new.is_null());

    (*new).size      = 0;
    (*new).alignment = 0;
    (*new).type_     = low::type_tag::STRUCT;
    (*new).elements  = elements;

    new
}

/// Creates a struct ffi_type with the given elements. Takes ownership
/// of the elements.
unsafe fn ffi_type_struct_create(elements: Vec<Type>) -> Owned<Type_> {
    ffi_type_struct_create_raw(ffi_type_array_create(elements))
}

/// Makes a copy of a type array.
unsafe fn ffi_type_array_clone(old: TypeArray_) -> Owned<TypeArray_> {
    let size = ffi_type_array_len(old);
    let new  = ffi_type_array_create_empty(size);

    for i in 0 .. size {
        *new.offset(i as isize) = ffi_type_clone(*old.offset(i as isize));
    }

    new
}

/// Makes a copy of a type.
unsafe fn ffi_type_clone(old: Type_) -> Owned<Type_> {
    if (*old).type_ == low::type_tag::STRUCT {
        ffi_type_struct_create_raw(ffi_type_array_clone((*old).elements))
    } else {
        old
    }
}

/// Destroys a TypeArray_ and all of its elements.
unsafe fn ffi_type_array_destroy(victim: Owned<TypeArray_>) {
    let mut current = victim;
    while !(*current).is_null() {
        ffi_type_destroy(*current);
        current = current.offset(1);
    }

    libc::free(victim as *mut libc::c_void);
}

/// Destroys a Type_ if it was dynamically allocated.
unsafe fn ffi_type_destroy(victim: Owned<Type_>) {
    if (*victim).type_ == low::type_tag::STRUCT {
        ffi_type_array_destroy((*victim).elements);
        libc::free(victim as *mut libc::c_void);
    }
}

impl Drop for Type {
    fn drop(&mut self) {
        unsafe { ffi_type_destroy(self.0.get_mut()) }
    }
}

impl Drop for TypeArray {
    fn drop(&mut self) {
        unsafe { ffi_type_array_destroy(self.0.get_mut()) }
    }
}

impl Clone for Type {
    fn clone(&self) -> Self {
        unsafe { Type(Unique::new(ffi_type_clone(*self.0))) }
    }
}

impl Clone for TypeArray {
    fn clone(&self) -> Self {
        unsafe {
            TypeArray(Unique::new(ffi_type_array_clone(*self.0)))
        }
    }
}

impl Type {
    /// Returns the representation of the C `void` type. This is only
    /// used for the return type of a Cif.
    pub fn void() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_void) })
    }

    /// Returns the unsigned 8-bit numeric type.
    pub fn u8() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_uint8) })
    }

    /// Returns the signed 8-bit numeric type.
    pub fn i8() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_sint8) })
    }

    /// Returns the unsigned 16-bit numeric type.
    pub fn u16() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_uint16) })
    }

    /// Returns the signed 16-bit numeric type.
    pub fn i16() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_sint16) })
    }

    /// Returns the unsigned 32-bit numeric type.
    pub fn u32() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_uint32) })
    }

    /// Returns the signed 32-bit numeric type.
    pub fn i32() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_sint32) })
    }

    /// Returns the unsigned 64-bit numeric type.
    pub fn u64() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_uint64) })
    }

    /// Returns the signed 64-bit numeric type.
    pub fn i64() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_sint64) })
    }

    /// Returns the C equivalent of Rust `usize` (`u16`).
    #[cfg(target_pointer_width = "16")]
    pub fn usize() -> Self {
        Self::u16()
    }

    /// Returns the C equivalent of Rust `isize` (`i16`).
    #[cfg(target_pointer_width = "16")]
    pub fn isize() -> Self {
        Self::i16()
    }

    /// Returns the C equivalent of Rust `usize` (`u32`).
    #[cfg(target_pointer_width = "32")]
    pub fn usize() -> Self {
        Self::u32()
    }

    /// Returns the C equivalent of Rust `isize` (`i32`).
    #[cfg(target_pointer_width = "32")]
    pub fn isize() -> Self {
        Self::i32()
    }

    /// Returns the C equivalent of Rust `usize` (`u64`).
    #[cfg(target_pointer_width = "64")]
    pub fn usize() -> Self {
        Self::u64()
    }

    /// Returns the C equivalent of Rust `isize` (`i64`).
    #[cfg(target_pointer_width = "64")]
    pub fn isize() -> Self {
        Self::i64()
    }

    /// Returns the C `float` (32-bit floating point) type.
    pub fn f32() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_float) })
    }

    /// Returns the C `double` (64-bit floating point) type.
    pub fn f64() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_double) })
    }

    /// Returns the C `void*` type, for passing any kind of pointer.
    pub fn pointer() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_pointer) })
    }

    /// Returns the C `long double` (extended-precision floating point) type.
    pub fn longdouble() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_longdouble) })
    }

    /// Returns the C `_Complex float` type.
    pub fn c32() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_complex_float) })
    }

    /// Returns the C `_Complex double` type.
    pub fn c64() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_complex_double) })
    }

    /// Returns the C `_Complex long double` type.
    pub fn complex_longdouble() -> Self {
        Type(unsafe { Unique::new(&mut low::ffi_type_complex_longdouble) })
    }

    /// Constructs a structure type whose fields have the given types.
    pub fn structure(fields: Vec<Type>) -> Self {
        unsafe {
            Type(Unique::new(ffi_type_struct_create(fields)))
        }
    }

    /// Constructs a structure type whose fields have the given types.
    pub fn structure_from_array(fields: TypeArray) -> Self {
        unsafe {
            Type(Unique::new(ffi_type_struct_create_raw(*fields.0)))
        }
    }

    /// Gets a raw pointer to the underlying
    /// [`ffi_type`](../low/struct.ffi_type.html).
    pub fn as_raw_ptr(&self) -> *mut low::ffi_type {
        *self.0
    }
}

impl TypeArray {
    /// Constructs an array the given `Type`s.
    pub fn new(elements: Vec<Type>) -> Self {
        unsafe { TypeArray(Unique::new(ffi_type_array_create(elements))) }
    }

    /// The length of this array of `Type`s.
    pub fn len(&self) -> usize {
        unsafe { ffi_type_array_len(*self.0) }
    }

    /// Gets a raw pointer to the underlying C array of
    /// [`ffi_type`](../low/struct.ffi_type.html)s.
    pub fn as_raw_ptr(&self) -> *mut *mut low::ffi_type {
        *self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_u64() {
        Type::u64();
    }

    #[test]
    fn clone_u64() {
        Type::u64().clone().clone();
    }

    #[test]
    fn create_struct() {
        Type::structure(vec![Type::i64(),
                             Type::i64(),
                             Type::u64()]);
    }

    #[test]
    fn clone_struct() {
        Type::structure(vec![Type::i64(),
                             Type::i64(),
                             Type::u64()]).clone().clone();
    }

}
