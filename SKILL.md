---
name: use-nosh
description: How to use the nosh CLI tool for managing todos and notes
argument-hint: "What do you need to do with nosh?"
---

# Using nosh

nosh is a terminal todo + notes manager. All data lives in a `.nosh/` directory.

## Running nosh

If working from the nosh repo:
```
cargo run --release --bin nosh -- <subcommand>
```

If installed globally:
```
nosh <subcommand>
```

Always run nosh from the directory where `.nosh/` lives (e.g. the notes vault root).

## Working directory

nosh reads/writes `.nosh/` in the current working directory. Set `$NOSH_DATA_DIR` to override.

Always run from the notes vault root where `.nosh/todos.json` and `.nosh/notes.json` live.

## Todos

### List todos
```
nosh todos list                # all unarchived todos
nosh todos list --pending      # only open/unfinished
nosh todos list --done         # only completed
nosh todos list --ids          # show numeric IDs (needed for other commands)
nosh todos list --archived     # show archived todos
```

Output format (without `--ids`):
```
[ ]  MM-DD HH:MM  Description here
[x]  MM-DD HH:MM  Another task
```

With `--ids`:
```
[ ]                  1  MM-DD HH:MM  Description here
[x]                  2  MM-DD HH:MM  Another task
```

### Create a todo
```
nosh todos create "Write the design doc"
```

### Mark as done
```
nosh todos do <id>
```

### Mark as not done
```
nosh todos undo <id>
```

### Edit description
```
nosh todos edit <id> "New description"
```

### Delete
```
nosh todos delete <id>
```

### Archive / unarchive
```
nosh todos archive <id>
nosh todos unarchive <id>
```

## Notes

### List notes
```
nosh notes list
```

Output:
```
1  07-13 14:30  Note title here
2  07-13 14:30  Another note
```

### View note content
```
nosh notes view <id>
```

Prints the raw markdown content to stdout.

### Create a note
```
nosh notes create "Note title"
```

Opens `$EDITOR` for the content. Not suitable for automated use.

### Edit a note
```
nosh notes edit <id>
```

Opens `$EDITOR` with existing content.

### Delete a note
```
nosh notes delete <id>
```

## Common workflows for agents

### Find all open todos and their IDs
```
nosh todos list --pending --ids
```

### Check what was recently completed
```
nosh todos list --done --ids
```

### Check archived work
```
nosh todos list --archived --ids
```

### Read all notes and scan for checkboxes
```
nosh notes list                              # get IDs and titles
nosh notes view <id>                         # get content of each
```

### Add a discovered action item as a todo
```
nosh todos create "Action item description"
```
