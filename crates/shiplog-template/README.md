# shiplog-template

Jinja2-like template system for packet rendering.

This crate provides a simple template engine for rendering shiplog packets with custom templates.

## Features

- Jinja2-like syntax for templates
- Variable substitution
- Conditional sections
- Loops over collections
- User-defined templates
- Integration with shiplog event schema

## Usage

```rust
use shiplog_template::{TemplateEngine, TemplateContext};
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;

let engine = TemplateEngine::new();
let template = "# {{ title }}\n\n{% for ws in workstreams %}## {{ ws.title }}\n{% endfor %}";

let mut context = TemplateContext::new();
context.set("title", "My Shipping Packet");
context.set("workstreams", &workstreams);

let rendered = engine.render(template, &context)?;
```

## Template Syntax

### Variables
```
{{ variable_name }}
```

### Conditionals
```
{% if condition %}
Content shown when condition is true
{% endif %}
```

### Loops
```
{% for item in collection %}
{{ item }}
{% endfor %}
```

## License

MIT OR Apache-2.0
