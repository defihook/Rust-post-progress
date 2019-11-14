# ex-post-progress

Gives you a progress bar for an existing program that's processing a file descriptor.

Usage:

```
$ ex-post-progress <pid> <file-path>
```

It will find the file descriptor which is `file-path` opened by `pid` automatically, and draw you a progress bar, exiting once the file decriptor has reached the end of the file.
