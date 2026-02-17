# shiplog-ports

Port traits for shiplog's hexagonal architecture.

## Core traits

- `Ingestor`: produces events + coverage.
- `WorkstreamClusterer`: groups events into workstreams.
- `Renderer`: renders packet output.
- `Redactor`: applies profile-based projections.

Adapters should depend on ports; ports should not depend on adapters.
