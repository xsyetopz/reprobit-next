---
description: Measure ReproBit disk use and free what rbit gc may remove
---

# Free disk space

Use the reprobit-disk skill. Run `rbit --json space`, then `rbit --json gc --dry-run`, report the largest items and what gc would free, and run `rbit --json gc` once the list holds only downloads, dropped tool versions, scratch and idle cache. Ask before `--images`.
