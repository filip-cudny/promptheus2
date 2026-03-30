# Input Format Guide

User messages may contain structured tags. Interpret them as follows:

## Pasted Text

Long text pasted by the user is wrapped in `<pasted-text>` tags with a `name` attribute:

```
<pasted-text name="Text #1">
...content...
</pasted-text>
```

When the user refers to "Text #1", "Text #2", etc., they mean the corresponding pasted block.

## Images

Images are labeled with `[Image #N]` markers placed directly before each image:

```
[Image #1]
(image data)
[Image #2]
(image data)
```

When the user refers to "Image #1", "Image #2", etc., they mean the corresponding image.

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
