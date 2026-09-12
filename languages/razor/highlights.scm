; Base C# highlighting, merged in from the official Zed C# extension
; (https://github.com/zed-extensions/csharp, grammar commit 485f0bae0274ac9114797fc10db6f7034e4086e3).
;
; tree-sitter-razor embeds the C# grammar directly (razor's own node-types.json
; contains class_declaration, method_declaration, binary_expression, etc.) rather
; than parsing @code blocks with a separate injected parser. Zed has no query
; "inherits" mechanism (that's an nvim-treesitter-only convention), so these
; patterns have to be copied in directly for C# nodes inside @code { ... } (and
; other embedded C#) to get highlighted at all.
;
; Keep this block in sync by hand if you bump the tree-sitter-c-sharp revision.

(identifier) @variable

; Methods
(method_declaration
  name: (identifier) @function)

(_
  function: (identifier) @function)

(local_function_statement
  name: (identifier) @function)

; Types
(interface_declaration
  name: (identifier) @type)

(class_declaration
  name: (identifier) @type)

(enum_declaration
  name: (identifier) @type)

(struct_declaration
  (identifier) @type)

(record_declaration
  (identifier) @type)

; TODO: Let this be @module again once we support fallbacks
(namespace_declaration
  name: (identifier) @type)

(generic_name
  (identifier) @type)

(type_parameter
  (identifier) @property.definition)

(parameter
  type: (identifier) @type)

(type_argument_list
  (identifier) @type)

(as_expression
  right: (identifier) @type)

(is_expression
  right: (identifier) @type)

(constructor_declaration
  name: (identifier) @constructor)

(destructor_declaration
  name: (identifier) @constructor)

(_
  type: (identifier) @type)

(base_list
  (identifier) @type)

(predefined_type) @type.builtin

; Enum
(enum_member_declaration
  (identifier) @property.definition)

; Literals
[
  (real_literal)
  (integer_literal)
] @number

[
  (character_literal)
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
  (interpolation_start)
  (interpolation_quote)
] @string

(escape_sequence) @string.escape

[
  (boolean_literal)
  (null_literal)
] @constant.builtin

; C# comments (// and /* */) inside @code blocks — distinct node type from
; razor_comment/html_comment below, so it needs its own rule.
(comment) @comment

; Tokens
[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "--"
  "-"
  "-="
  "&"
  "&="
  "&&"
  "+"
  "++"
  "+="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "|"
  "|="
  "||"
  "?"
  "??"
  "??="
  "^"
  "^="
  "~"
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  ":"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  (interpolation_brace)
] @punctuation.bracket

; Keywords
[
  (modifier)
  "this"
  (implicit_type)
] @keyword

[
  "add"
  "alias"
  "as"
  "base"
  "break"
  "case"
  "catch"
  "checked"
  "class"
  "continue"
  "default"
  "delegate"
  "do"
  "else"
  "enum"
  "event"
  "explicit"
  "extern"
  "finally"
  "for"
  "foreach"
  "global"
  "goto"
  "if"
  "implicit"
  "interface"
  "is"
  "lock"
  "namespace"
  "notnull"
  "operator"
  "params"
  "return"
  "remove"
  "sizeof"
  "stackalloc"
  "static"
  "struct"
  "switch"
  "throw"
  "try"
  "typeof"
  "unchecked"
  "using"
  "while"
  "new"
  "await"
  "in"
  "yield"
  "get"
  "set"
  "when"
  "out"
  "ref"
  "from"
  "where"
  "select"
  "record"
  "init"
  "with"
  "let"
] @keyword

; pattern expression keywords
(negated_pattern
  "not" @keyword)

(and_pattern
  "and" @keyword)

(or_pattern
  "or" @keyword)

; scoped keyword
(scoped_type
  "scoped" @keyword)

; Attribute
(attribute
  name: (identifier) @attribute)

; Parameters
(parameter
  name: (identifier) @variable.parameter)

; Type constraints
(type_parameter_constraints_clause
  (identifier) @property.definition)

; Method calls
(invocation_expression
  (member_access_expression
    name: (identifier) @function))

; ---------------------------------------------------------------------------
; Razor-specific highlighting below (your original queries, unchanged order
; so they override the generic C# patterns above for the same nodes).
; ---------------------------------------------------------------------------

[
  (razor_comment)
  (html_comment)
] @comment

[
  "at_page"
  "at_using"
  "at_model"
  "at_rendermode"
  "at_inject"
  "at_implements"
  "at_layout"
  "at_inherits"
  "at_attribute"
  "at_typeparam"
  "at_namespace"
  "at_preservewhitespace"
  "at_block"
  "at_at_escape"
  "at_colon_transition"
  "at_addtaghelper"
  "at_removetaghelper"
  "at_taghelperprefix"
] @constant.macro

[
  "at_lock"
  "at_section"
] @keyword

[
  "at_if"
  "at_switch"
] @keyword.conditional

[
  "at_for"
  "at_foreach"
  "at_while"
  "at_do"
] @keyword.repeat

[
  "at_try"
  "catch"
  "finally"
] @keyword.exception

[
  "at_implicit"
  "at_explicit"
] @variable

(razor_implicit_expression
  "at_implicit" @keyword.coroutine
  (await_expression
    "await" @keyword.coroutine))

(razor_rendermode) @property

(razor_attribute_name) @function

(taghelper_wildcard) @character.special
