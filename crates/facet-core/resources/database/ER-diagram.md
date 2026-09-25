# Database ER diagram

Entity-relationship diagram of the SQLite schema defined by the `NNN_*.sql` files in this
directory. Generated from the DDL; keep it in sync when the schema changes.

Foreign keys (referencing → referenced):

- `device_event.event_type_id` → `event_type`
- `device_notification.event_type_id` → `event_type`
- `category.icon_id` → `icon`
- `category.colour_id` → `colour`
- `category.project_id` → `project`
- `face.category_id` → `category`
- `time_entry.category_id` → `category`
- `time_entry.device_event_id` → `device_event`
- `device_event.timezone_id` → `timezone`
- `device_notification.timezone_id` → `timezone`
- `time_entry.start_timezone_id` → `timezone`
- `time_entry.end_timezone_id` → `timezone`
- `debug_log.timezone_id` → `timezone`

`setting` is the one table with no foreign keys, so it is shown but unconnected.

```mermaid
erDiagram
    event_type ||--o{ device_event : "classifies"
    event_type ||--o{ device_notification : "classifies"
    icon ||--o{ category : "shown by"
    colour ||--o{ category : "coloured by"
    project ||--o{ category : "groups"
    category ||--o{ face : "assigned to"
    category ||--o{ time_entry : "accrues"
    device_event ||--o{ time_entry : "derived from"
    timezone ||--o{ device_event : "captured in"
    timezone ||--o{ device_notification : "captured in"
    timezone ||--o{ time_entry : "captured in"
    timezone ||--o{ debug_log : "captured in"

    event_type {
        INTEGER event_type_id PK
        TEXT    event_name
    }

    device_event {
        INTEGER device_event_id PK
        INTEGER event_number
        INTEGER event_type_id FK
        INTEGER device_face
        TEXT    start_time
        INTEGER timezone_id FK
        INTEGER start_epoch
        REAL    duration_seconds
        INTEGER paused
        INTEGER finalised
        INTEGER processed
    }

    device_notification {
        INTEGER device_notification_id PK
        INTEGER event_type_id FK
        TEXT    start_time
        INTEGER timezone_id FK
        INTEGER start_epoch
        TEXT    payload
    }

    icon {
        INTEGER icon_id PK
        TEXT    icon_name
    }

    colour {
        INTEGER colour_id PK
        TEXT    colour_name
        TEXT    device_hex
        INTEGER white_lines
    }

    category {
        INTEGER category_id PK
        TEXT    category_name
        INTEGER icon_id FK
        INTEGER colour_id FK
        INTEGER project_id FK
        INTEGER daily_limit
        INTEGER cost
    }

    face {
        INTEGER face_id PK
        INTEGER category_id FK
        INTEGER locked
    }

    time_entry {
        INTEGER time_entry_id PK
        INTEGER category_id FK
        INTEGER device_event_id FK
        TEXT    started_at
        INTEGER start_timezone_id FK
        TEXT    ended_at
        INTEGER end_timezone_id FK
        REAL    duration_seconds
        INTEGER total_cost
        INTEGER synced_to_google_calendar
    }

    setting {
        INTEGER setting_id PK
        TEXT    setting_name
        TEXT    setting_value
        TEXT    setting_description
    }

    debug_log {
        INTEGER debug_log_id PK
        TEXT    logged_at
        INTEGER timezone_id FK
        TEXT    tag
        TEXT    message
    }

    project {
        INTEGER project_id PK
        TEXT    project_name
    }

    timezone {
        INTEGER timezone_id PK
        TEXT    timezone_name
        TEXT    display_name
        INTEGER active
    }
```
