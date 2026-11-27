# Copy the universal skills from plugins/reprobit into each harness plugin. Harness plugins
# ship their own copies because a plugin installs as one self-contained folder; skills only a
# harness has, such as reprobit-team, stay as they are.
[doc("Copy the universal ReproBit skills into each harness plugin.")]
plugins-sync:
    for plugin in plugins/reprobit-*/; do \
      for skill in plugins/reprobit/skills/*/; do \
        rsync -a --delete "$skill" "$plugin/skills/$(basename "$skill")/"; \
      done; \
    done
