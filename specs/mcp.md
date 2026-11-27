# MCP

`rbit mcp` serves the commands to agents. Every reply enters an agent's context, and
harnesses persist or cut off large tool results, so a reply stays inside a fixed budget and
says what it changes.

## MCP-1 One copy of the result

A tool reply carries its JSON result once, as compact text, with no `structuredContent` copy.

Given a set-up project
When an agent calls the `verify` tool
Then the reply's one text item parses to the result, with `identical` true, and the reply has no `structuredContent`

## MCP-2 The reply budget

A result over 22,000 characters, about 10,000 tokens of dense JSON, is replaced by an error that names its size, the tool's optional arguments, and the `rbit --json` command to run instead.

Given a library with 400 members that `locate --all` lists one by one
When an agent calls `locate` with `all` true
Then the reply is an error of at most 22,000 characters that names `rbit --json locate` and the `all` argument

## MCP-3 What a tool changes

Each tool declares `readOnlyHint`, and a tool that deletes what it cannot rebuild also declares `destructiveHint`, so a harness can approve read-only tools alone.

Given the tool list
When an agent reads its annotations
Then `status`, `diff`, `locate` and `space` are read-only, `score` and `setup` are not, and `gc` and `tool_remove` are destructive
