# Disk

A matching project collects compilers, container images, build copies, and caches. On a
laptop with 128 or 256 GB of storage the operating system already holds a large share, so
ReproBit lists everything it keeps and removes only what it can fetch or rebuild.

## `rbit space [DIR] [--keep-days N]`

Lists each item ReproBit holds, largest first: installed tool versions, downloaded archives,
runner images, and, when `DIR` holds a `reprobit.toml`, the project's `.reprobit/` scratch
and step cache. Each item has `bytes`, and a `reclaim` reason when `rbit gc` may remove it.
`reclaimable` totals what plain `gc` frees; `reclaimable_images` adds what `gc --images`
frees.

## `rbit gc [DIR] [--keep-days N] [--images] [--dry-run]`

| Removed | Why it is safe |
| --- | --- |
| Downloaded archives | Kept only to reinstall; the catalog's URL and digest fetch them again |
| Tool versions the catalog no longer pins | No project built from this catalog uses them |
| Project scratch: `stage`, `verify`, `grind`, `reports`, `logs` under `.reprobit/` | The next build or verify writes them again |
| Step-cache entries unused for more than `--keep-days` days (default 14) | A miss rebuilds the step |

A step-cache hit refreshes the entry's `.used` marker, so age counts from last use.

`--images` also removes runner images (`reprobit-wine:*`) that a newer image replaced.
Docker and Podman storage is shared with other work, so images go only with this flag.
`--dry-run` lists what would go and removes nothing.

A tool a project still pins stays installed; `rbit tool remove <id>` deletes it, and
`rbit tool remove --all` deletes everything under `rbit tool home`.

## Outside ReproBit

`rbit gc` leaves alone what other tools or people made: permuter output, copies under a
project's `scratch/`, Docker build cache and stopped containers (`docker system df` shows
them), and disc images or ROM dumps kept beside a project.
