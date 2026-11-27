---
name: reprobit-disk
description: Keep a decompilation's disk use small. Measure with rbit space, then free downloads, dropped tool versions, scratch, and idle step cache with rbit gc. Use when disk runs low.
license: LGPL-3.0-only
---

# Disk hygiene

Matching projects collect compilers, container images, build copies, and caches. On a
128 or 256 GB disk the operating system already holds 20 to 30 GB, so check before you fill it.

## Measure

```sh
rbit --json space            # in a project directory, includes its .reprobit state
```

Each item has `bytes` and, when `rbit gc` may remove it, a `reclaim` reason. `reclaimable` is
what plain `gc` frees; `reclaimable_images` is what `gc --images` adds.

## Free

1. `rbit --json gc --dry-run` lists what would go.
2. `rbit --json gc` removes downloaded archives, tool versions the catalog no longer pins,
   project scratch (`stage`, `verify`, `grind`, `reports`, `logs`), and step-cache entries
   unused for 14 days (`--keep-days N` changes that).
3. `rbit --json gc --images` also removes superseded runner images. Docker storage is shared
   with other work, so ask before running it.
4. A tool the project no longer uses stays installed until you remove it:
   `rbit --json tool remove <id>`.

## Outside ReproBit

Report these; they belong to the user:

- Permuter output directories and `scratch/` copies inside the project.
- `docker system df` shows build cache and stopped containers that other tools left.
- Extracted disc images or ROM dumps kept beside the project.
