//! Error types for HTTP field parsing.

/// Error returned when parsing a full HTTP field line fails.
#[derive(Debug, PartialEq)]
pub struct ParseFieldError;

impl_more::impl_display!(ParseFieldError: "Invalid HTTP field line");
impl_more::impl_leaf_error!(ParseFieldError);

/// Error returned when parsing an HTTP field name fails.
#[derive(Debug, PartialEq)]
pub struct ParseFieldNameError;

impl_more::impl_display!(ParseFieldNameError: "Invalid HTTP field name");
impl_more::impl_leaf_error!(ParseFieldNameError);

/// Error returned when parsing an HTTP field value fails.
#[derive(Debug, PartialEq)]
pub struct ParseFieldValueError;

impl_more::impl_display!(ParseFieldValueError: "Invalid HTTP field value");
impl_more::impl_leaf_error!(ParseFieldValueError);
