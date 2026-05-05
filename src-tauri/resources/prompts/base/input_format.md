# Input Format Guide

User messages may contain structured tags. Interpret them as follows:

## Context

The system message may include a `<context>` block — persistent background information the user has attached to this conversation. Treat it as reference material that applies to every message in the conversation:

```
<context>
...background info, notes, reference material...
</context>
```

Context may also include images, labeled `[Context Image #N]`. These appear at the beginning of the first user message and remain relevant throughout the conversation.

## Pasted Text

Long text pasted by the user is wrapped in `<pasted-text>` tags with a `name` attribute:

```
<pasted-text name="Text #1">
...content...
</pasted-text>
```

When the user refers to "Text #1", "Text #2", etc., they mean the corresponding pasted block.

## Images

There are two kinds of images:

**Context images** are attached to the conversation context (see above). They are labeled:

```
[Context Image #1]
(image data)
[Context Image #2]
(image data)
```

**Pasted images** are attached directly to a specific message. They are labeled:

```
[Image #1]
(image data)
[Image #2]
(image data)
```

When the user refers to "Context Image #1", "Image #2", etc., they mean the corresponding image. Context images persist across the entire conversation; pasted images belong to the message they appear in.

## Skills

Skills are prompt templates invoked by the user. They appear as:

```
<skill name="skill-name">
...skill instructions...
</skill>

<input>
...user input...
</input>
```

Follow the skill instructions using the provided input.
