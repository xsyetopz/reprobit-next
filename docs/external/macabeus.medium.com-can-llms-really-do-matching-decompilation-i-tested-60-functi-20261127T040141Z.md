---
title: "Can LLMs Really Do Matching Decompilation? I Tested 60 Functions to Find Out | by Bruno Macabeus | Medium"
url: "https://macabeus.medium.com/can-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288"
canonical: "https://macabeus.medium.com/can-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288"
captured_at: "2026-11-27T04:01:41.311Z"
adapter: "generic"
scope: "article"
site: "Medium"
language: "en"
coverage:
  "loaded_content_only": true
  "nodes": 1106
  "blocks": 279
  "partial": false
content_trust: "untrusted"
---

# Can LLMs Really Do Matching Decompilation? I Tested 60 Functions to Find Out | by Bruno Macabeus | Medium

> Can LLMs Really Do Matching Decompilation? I Tested 60 Functions to Find Out In the previous chapter, we dove deep into the Kappa implementation, our decompiler buddy who lives in VS Code. It adds …

Reverse Engineering

[Reverse Engineering](https://medium.com/tag/reverse-engineering?source=post_page---header_tags--4e39b0ae4288------------------------------------------)

Retro Gaming

[Retro Gaming](https://medium.com/tag/retro-gaming?source=post_page---header_tags--4e39b0ae4288------------------------------------------)

LLM

[LLM](https://medium.com/tag/llm?source=post_page---header_tags--4e39b0ae4288------------------------------------------)

Benchmark

[Benchmark](https://medium.com/tag/benchmark?source=post_page---header_tags--4e39b0ae4288------------------------------------------)

Open Source

[Open Source](https://medium.com/tag/open-source?source=post_page---header_tags--4e39b0ae4288------------------------------------------)

# Can LLMs Really Do Matching Decompilation? I Tested 60 Functions to Find Out

[Image: Bruno Macabeus](https://miro.medium.com/v2/resize:fill:64:64/0*0hCeIeN8OW9pv8ZS.jpg)

[https://macabeus.medium.com/?source=post\_page---byline--4e39b0ae4288------------------------------------------](https://macabeus.medium.com/?source=post_page---byline--4e39b0ae4288------------------------------------------)

Bruno Macabeus

[Bruno Macabeus](https://macabeus.medium.com/?source=post_page---byline--4e39b0ae4288------------------------------------------)

17 min readMar 10, 2026

\[clap\]

[\[clap\]](https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fvote%2Fp%2F4e39b0ae4288%26operation=register%26redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288%26user=Bruno+Macabeus%26userId=3d9d80444f9d%26source=---header_actions--4e39b0ae4288---------------------clap_footer---------------------)

\--

[https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F\_%2Frepost%2Fp%2F4e39b0ae4288\&operation=register\&redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288\&user=Bruno+Macabeus\&userId=3d9d80444f9d\&source=---header\_actions--4e39b0ae4288---------------------repost\_header---------------------](https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Frepost%2Fp%2F4e39b0ae4288%26operation=register%26redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288%26user=Bruno+Macabeus%26userId=3d9d80444f9d%26source=---header_actions--4e39b0ae4288---------------------repost_header---------------------)

\--

\[Add to list bookmark button\]

[\[Add to list bookmark button\]](https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fbookmark%2Fp%2F4e39b0ae4288%26operation=register%26redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288%26source=---header_actions--4e39b0ae4288---------------------bookmark_footer---------------------)

[https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2Fplans%3Fdimension%3Dpost\_audio\_button%26postId%3D4e39b0ae4288\&operation=register\&redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288\&source=---header\_actions--4e39b0ae4288---------------------post\_audio\_button---------------------](https://medium.com/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2Fplans%3Fdimension%3Dpost_audio_button%26postId%3D4e39b0ae4288%26operation=register%26redirect=https%3A%2F%2Fmacabeus.medium.com%2Fcan-llms-really-do-matching-decompilation-i-tested-60-functions-to-find-out-4e39b0ae4288%26source=---header_actions--4e39b0ae4288---------------------post_audio_button---------------------)

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:2000/format:webp/1*fNuRGlJkgETF8U_NhjSRhw.png)

In the previous chapter, we dove deep into the Kappa implementation, our decompiler buddy who lives in VS Code. It adds AI-powered features for decompilation and automates boring flows when decompiling a single function, as we did manually in the second chapter.

[previous chapter](https://gambiconf.substack.com/p/development-journey-on-game-decompilation-808)

[second chapter](https://gambiconf.substack.com/p/development-journey-on-game-decompilation-841)

But now we have the question: are LLMs really good for matching decompilation?

> ⚙️ What is Matching Decompilation? Matching decompilation is the art of converting assembly back into C source code that, when compiled, produces byte-for-byte identical machine code. It’s popular in the retro gaming community for recreating the source code of classic games. For example, Super Mario 64 and The Legend of Zelda: Ocarina of Time have been fully match-decompiled.
>
> [Super Mario 64](https://github.com/n64decomp/sm64)
>
> [The Legend of Zelda: Ocarina of Time](https://github.com/zeldaret/oot)

Decompilation has been around for a long time. Many retro games were decompiled before AI became a thing. For years the community built solid tools to help decompile functions. But they all share one thing: they are procedural.

Beyond that, some people reasonably don’t believe that LLMs are good for decompilation. I’ve received some fair criticism because of a lack of data.

So, to know whether LLMs are good for decompilation or not, we need to measure their output on real, in-progress decompilation projects.

It’s worth mentioning that LLMs don’t exclude the great programmatic tooling the community has built over the years. They join forces with LLMs.

So, the goal is to measure the outcomes of matching decompilation in a pipeline using both programmatic and AI-powered tools. The pipeline can start using programmatic tools to decompile a function and then use an LLM to try matching it.

More importantly, having a solid way to benchmark decompilation tasks is a requirement to pave the way to improve both the AI-powered and programmatic decompilation tools with evidence-based decisions. As I described briefly in the previous chapter, sometimes it was difficult to know if a given change to the prompt improved it or was actually harming it.

It’s worth mentioning Chris Lewis’ work. A few months ago, he started a decompilation project for Snowboard Kids 2 (SK2). He’s been sharing his progress and insights on his blog. During his journey, he’s been building an agentic flow to automatically decompile this game. He shared in the 2nd chapter how it sped up a lot of his work.

[on his blog](https://blog.chrislewis.au/)

[in the 2nd chapter](https://blog.chrislewis.au/the-unexpected-effectiveness-of-one-shot-decompilation-with-claude/)

Although it’s an engaging finding and evidence that LLMs are really helpful, there’s no benchmark data comparing the same dataset of functions.

This gap is what we are going to fill in this chapter!

Spoiler: Across 60 functions from two real decomp projects, the pipeline matches 74% functions, and 88% of them produce the same outcome every time!

## Mizuchi enters the scene

So, the next goal is to produce benchmarks. Initially, I spent some time trying to find an off-the-shelf tool, but at last I decided to make a tailored pipeline runner for decompilation that outputs benchmarking data.

[Image](https://miro.medium.com/v2/resize:fit:600/0*U0xbSakNR7EMUxds.png)

Mizuchi — Forge C from the ashes of assembly

I named it Mizuchi, just to keep on-brand with Kappa (both kappa and mizuchi are mythological creatures in Japanese folklore). Also, I picked a creative name because I don’t know how Mizuchi will grow up, so I avoided something too tied to the current goal. There is a chance that this tooling will become useful for more cases than only matching decompilation (I’ll talk more about it later!).

[Mizuchi](https://github.com/macabeus/mizuchi)

Mizuchi runs the following plugin-based pipeline.

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:2000/0*s8J1wYabMqy5ab_Z.png)

These are the current plugins available:

- Get Context: Runs a shell script to get the context from a given function, likely using the m2ctx.py script. It writes a C file that includes structs, macros, and other definitions that are prepended to the C code with the attempt to decompile the function.
- m2c: Calls m2c (a machine code to C decompiler) to programmatically produce C code from the target assembly function.

  [m2c](https://github.com/matt-kempster/m2c)
- Compiler: Compiles the generated C.
- Objdiff: Compares the compiled object file against the target one, using objdiff.

  [objdiff](https://github.com/encounter/objdiff)
- Permuter: Calls decomp-permuter to programmatically permute C code to find a better matching rate.

  [decomp-permuter](https://github.com/simonlindholm/decomp-permuter)
- Claude Runner: It’s the most complex plugin. It runs a Claude Agent SDK session to decompile the target function.

Before starting the pipeline, we have Tasks Loader. It reads a folder that must contain the following structure to load the tasks we are going to run:

```
tasks/  my-function-1/    prompt.md         # The prompt content    settings.yaml     # Metadata (functionName, targetObjectPath, asm)  my-function-2/    prompt.md    settings.yaml  ...
```

Then, we have Plugin Manager, which runs the plugins in three phases:

1. Setup Phase: Just calls the Get Context plugin.
2. Programmatic Phase: It’s a one-way flow with the programmatic tools: m2c → Compiler → Objdiff → Decomp Permuter. m2c produces a first attempt to decompile a function, then calls Compiler and Objdiff. It may then call Permuter, which iterates through code permutations. If there is no perfect match, it goes to the AI-Powered Phase.
3. AI-Powered Phase: It’s a loop including Claude Runner → Compiler → Objdiff. The result from m2c is included in the initial prompt for Claude Runner. This phase runs Permuter as a background task. Whenever Claude finds a better match, it spawns a background job with Permuter to iterate the code produced by Claude. We use the result from Permuter only if it finds a perfect match, and in this case, the pipeline is finished.

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*AENYfQMkV_Zo8Y32.png)

Run report for a function that matched in the Programmatic Phase

### Claude Runner Plugin

Since it’s the most complex plugin and where most of the magic happens, I need to explain it in more detail.

The agent runs in a sandboxed mode. Claude cannot modify project files, build scripts, or configuration. It can only submit C code through the pipeline’s designated interface.

The agent includes an MCP tool to compile code and see the assembly, so it can play a bit before submitting an attempt. But to avoid it spending too much time just calling the tool without submitting an attempt, there is a configurable limit on tool calls per submission.

Also, there are three different time-outs:

- Time-to-first-token (TTFT): how long it takes until we receive the first token from the server. After the first token is received, the timer for soft time-out and hard time-out starts. If no token is received, abort the session and start a new one on the next attempt.
- Soft time-out: If Claude didn’t submit an attempt within this timeframe, send a message asking to submit an answer now.
- Hard time-out: If no answer is submitted, stop the session.

The specific values used for this benchmark are listed in the next section.

If an attempt fails, we submit a follow-up message to try again until the limit is reached. Every failed attempt counts, even if it was a TTFT time-out. The follow-up message includes the compiler error, objdiff result report, and, in the case of a hard timeout, a message saying to quickly submit an attempt.

## Benchmark Setup and Goals

### Projects

For the benchmark report, I’m using two decomp projects in a specific version:

- Sonic Advance 3 (SA3): Game Boy Advance game with 35.4% of the indexed functions decompiled

  [Sonic Advance 3](https://github.com/SAT-R/sa3/tree/b0321cdc57ef829b24d4179ed625cb3403e26633)
- Animal Forest (AF): Nintendo 64 game compiled with IDO and with 24% of the indexed functions decompiled

  [Animal Forest](https://github.com/zeldaret/af/tree/ff5f68aacc7aab10d5b281277447f1b805c0a5a2)

"Indexed functions" means that it includes only the functions that are indexed by Mizuchi. Then, the matching progress metric and number of functions here are different from the ones officially used in the projects.

I initially planned to test using SK2 instead of AF, but I replaced it because SK2 is a late-stage decompilation project with few easy functions to decompile. A good benchmark needs functions from easy to hard, and we can find those in SA3 and AF, but not in SK2.

### Functions Scoring

Okay, now, how do you pick which functions to test? Let’s talk briefly about the function scoring algorithm. Mizuchi segments the functions into three percentiles: easy, medium, and hard.

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*V1Tzy1r07kmLwhiZ.png)

Function Scoring tab on Mizuchi Atlas

I started with the idea written by Lewis here and his code here. The scores are computed by a logistic regression trained on the project’s codebase using a set of assembly features. It assumes functions that have already been decompiled are, on average, easier.

[here](https://blog.chrislewis.au/the-unexpected-effectiveness-of-one-shot-decompilation-with-claude/)

[here](https://github.com/cdlewis/snowboardkids2-decomp/blob/d75d2fd4c1853f1b7883ba741dac797db9930137/tools/score_functions.py)

This scoring algorithm doesn’t include two relevant characteristics that likely help AI and are included in the prompt: the number and proximity of the examples and the type definitions.

In any case, a perfect scoring algorithm isn’t needed. I’m only using it to select a well-distributed set of functions for the benchmark. So, I picked 10 from each percentile: 10 easy, 10 medium, and 10 hard. The total is 60 functions (30 from SA3 and 30 from AF).

I also reduced the scope for Mizuchi here. For SA3, I filtered to only Thumb functions, since m2c and agbcc (the GBA compiler) only work well for Thumb instructions. And for AF, I filtered to only the functions for which I could programmatically generate a valid context file using m2ctx.py.

### Prompt Builder

Now that we’ve picked the functions, the next step is building a tailored prompt for each one. It’s the system prompt used for each function.

The prompts used in the benchmark were crafted using Mizuchi, with code ported from Kappa. As explained in the previous chapter, the prompt builder includes

[previous chapter](https://gambiconf.substack.com/p/development-journey-on-game-decompilation-808)

- A general context for the task (the platform, architecture, goal)
- Up to 5 examples from similar functions that have already been decompiled. The similarity is based on an embedded vector from the assembly code, using a normalized asm code and the jina-embeddings-v2-base-code model.

  [jina-embeddings-v2-base-code](https://huggingface.co/jinaai/jina-embeddings-v2-base-code)
- Signature of decompiled functions that call the target one.
- C declaration of the target function.
- Signature of the functions that are called by the target one.
- Type definitions likely used by the target function.

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*TZhFARy6VML9gnpT.png)

Prompt Builder tab on Mizuchi Atlas

### Setting the goal and parameters

In the benchmark, we want to evaluate

- How well the programmatic tools (m2c + decomp permuter) can perform alone.
- How well the LLM-powered tooling performs for this task in addition to the programmatic tools.

I’ll test using only Claude Sonnet 4.6.

I’ll run it 3 times per dataset to reduce the noise from the random-walk nature of LLMs.

For this benchmarking, I’ve set a limit of 12 attempts per function. It’s a low number, but necessary because each attempt makes the run take longer, and I’m already burning many tokens on this experiment.

During the development, I noticed that some functions only pass after 20 attempts. Anecdotally, Lewis commented that one function required 87 attempts before Claude finally succeeded.

The time-outs are also very aggressive:

- TTFT: 3 minutes.
- Soft time-out: 7 minutes.
- Hard time-out: 10 minutes (= 3 minutes to produce a final code after the soft time-out)

Still, I think these values are good enough for this benchmarking. It’ll evaluate a more adversarial scenario for the LLM, where we don’t have much budget.

You can check here the full Mizuchi code that was used to run this benchmark, including the prompts and settings for both SA3 and AF and the script to get the functions randomly per tier. You should be able to reproduce the benchmark on your own after cloning and setting up SA3 and AF. You can even tweak the settings if you’d like to. Mizuchi is highly configurable!

[here](https://github.com/macabeus/mizuchi-benchmark-report-sa3-af/tree/main)

So, it’s all set! Let’s run the pipeline.

### Running the pipeline

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*TIdXl-H4hlrxlmjf.png)

It took \~10 days to produce the 6 benchmark runs with results I’m confident sharing. There were many small issues I only noticed after the fact, and I needed to discard everything and run it again.

Also, it’s worth mentioning that this benchmark isn’t evaluating the LLM in a vacuum. It’s running on top of Mizuchi and in a specific timeframe, so infrastructure issues from both Mizuchi and Anthropic might affect the results.

The most time-consuming issue was instability in the Claude service. I noticed that, sometimes, even when it’s all green on their status page, the tokens per second are too low and we have timeouts.

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*ZQmkCWZ5Ub8S4hy7.png)

The first two SA3 runs had significantly higher throughput than the 3rd. The first two runs had fewer timeouts and a higher match rate, though the overall function outcome (matched vs. not) was largely consistent. For AF, all benchmark runs used the same Mizuchi version and settings.

## Get Bruno Macabeus’s stories in your inbox

Join Medium for free to get updates from this writer.

Remember me for faster sign in

Because of the instability of the API, the 3rd SA3 run and all AF runs include additional safeguards, such as an explicit TTFT timeout that aborts and restarts on unresponsive API calls.

## Results

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*PlAGbUGK3y_Fny-d.png)

Let’s explore the benchmark result!

> You can explore the full report on this webapp. Check the full results in this repository.
>
> [on this webapp](https://macabeus.github.io/mizuchi-benchmark-reports/)
>
> [this repository](https://github.com/macabeus/mizuchi-benchmark-reports)

Yay! It’s all done now! A few highlights:

- The SA3 Run 2 had the best success rate: 80% of the functions were matched. The average success rate across the 6 runs is 74%.
- Permuter played well with the code generated by Claude on AF: it helped match 7 functions across the 3 runs.
- The pipeline is very deterministic: 88% of the functions had identical outcomes across all runs.

### Attempts to Match

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*UnBl0anEwWDha5KJ.png)

Percentile chart on how many functions are matched by attempt N

When the function still wasn’t matched in the Programmatic Phase, nearly half of the matches happened on the first attempt of the AI-Powered Phase. If a function isn’t matched by the 3rd attempt, the conditional probability of eventually matching drops to roughly one-quarter.

Curiously, there was exactly one case where a function matched at the 12th attempt. It was likely just an outlier of luck, since in the other runs it failed to match.

### Programmatic Tools

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*MOfxvuaMc697Ouo4.png)

How a match was achieved in both games

For SA3, no function was matched using only programmatic tools. AF was different. Permuter and m2c were great allies on AF! m2c produced matching code for 4 functions per run, and Permuter matched 3–4 functions per run.

Permuter likely worked better on AF because it was initially designed for IDO (the compiler used for AF). Although there are efforts to make it work with other compilers, IDO is still the one it works best with. It’s likely the same reason for m2c: the support for ARM was released 6 months ago, so it isn’t as mature as the support for IDO.

Permuter mostly found matches using the code generated by Claude. The only exception is func\_808D6BAC\_jp, where it found the match still in the Programmatic Phase, using the m2c code as the base. Curiously, in Run 1 and Run 3, Permuter helped to match the function func\_808D6BAC\_jp while in Run 2, no match was achieved for this function.

### Scoring

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*i9sIErR54qyMT72c.png)

Match Rate for SA3 per tiers

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*phQ8uVsrBdhg4m_B.png)

Match Rate for AF per tiers

I also tried to check whether the scoring mechanism that splits functions into three tiers (easy/medium/hard) really offered predictability.

It worked well for AF: the easy functions matched more than the medium ones, which, in turn, matched more than the hard ones.

On the other hand, it didn’t work well for SA3. The medium functions matched more than the easy ones.

### API Degradation

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*UD6692Oa-_CmipjB.png)

Tokens/seconds for SA3 runs

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*DUGdP__0eMAef1cg.png)

Tokens/seconds for AF

As you can see, the first 2 SA3 runs are way faster than the 3rd SA3 run and all AF runs. It’s likely not from my side but something Anthropic changed in their model or an infrastructure change on their end, since my dataset and environment are the same.

The first two runs had a much lower average attempt count and fewer timeouts. It happened because of an API latency degradation: throughput dropped from 57 tokens/second to 11 tokens/second (5x), causing 95 soft timeouts that cascaded into 59 compile failures, since code produced under soft-timeout pressure usually has compilation errors.

In the latest run for SA3 and in all runs for AF, we had timeouts on hard functions. In some cases, they were severe, producing no useful output, as happened with sub\_80720E4 on SA3. TTFT within 3 minutes as a timeout and 10 minutes for a hard timeout might be too small for very complex functions.

### Flipped Functions

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*Fk8nLbaNq3YwTba-.png)

Function that flipped in both games

In most cases, the result per function is stable per run. A function that matches in one run will typically match in the others, and vice versa. Still, there are curious cases where flips happen. There were 5 functions that flipped on SA3 and only 2 functions that flipped on AF.

More flipped functions on SA3 are expected, since the tok/s drop only affected the last run. For example, the function sub\_8085618 on SA3 failed to match only in the last run, likely because of the low tok/s that repeatedly triggered the soft timeout.

So, to compare stability between runs, we can rely more on AF, and it’s mostly stable, since only two functions flipped: func\_800BC368\_jp matched only in Run 2 on the first attempt and in Run 3 on the 7th attempt, while func\_808D6BAC\_jp ⁣matched only when Permuter found the match.

In the end, we can conclude that the pipeline is very deterministic! 53 of the 60 functions (88%) had identical outcomes across all three runs. The LLM’s randomness affects mostly the attempt count and path, but not the outcome.

### Ceiling

We have 12 functions that always fail (5 on SA3 + 7 on AF). These functions are the frontier. That’s what we need to solve to have a fully automated decompilation flow.

In some cases, it was purely timeouts, but in other cases, Claude still returned compilable code but could not find a perfect match. These always-failing functions point to specific areas where Mizuchi can improve.

## Future Improvements

A key decision early in the project was outputting detailed benchmark reports in JSON, since it allowed me to feed them to Claude Code and ask it to analyze, explain, and provide insights.

It helped me a lot during development. For example, limiting the number of tool calls to ‘compile and verify’ and adding a soft timeout were proposed by Claude Code after reading the reports. Claude Code helped both identify room for improvement and address these issues, then verified from a new report whether the fixes worked.

Still, there are many other improvements that can be made to Mizuchi. Some of them:

### Refinements on the MCP tools

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*E0UlYG8z9Z6dCf6A.png)

Example from one of the conversation logs when Claude says that it matches, but it isn’t

Currently, we have only the tool compile\_and\_view\_assembly. There is no way for the agent to run objdiff before submitting the attempt. The agent does its best to compute the diff, but frequently it wrongly assumes there is a perfect match when there isn’t.

Thus, I think it would be a good improvement to add the capability to see the diff, maybe on this same MCP tool.

### Better stall action

After 3 attempts without an improvement, it triggers a stall detection. But for now, the only action is appending a message to the prompt saying to think deeper and reminding it of the last best attempt.

It doesn’t do much. It needs a better strategy, like switching to a smarter model, giving more time to think, or even giving up early to save tokens.

### Avoid Claude submitting the same code

Sometimes, Claude is stubborn and submits the same code. After analyzing the logs, it happened 49 times.

For example, in AF Run 2, when working on func\_808D6BAC\_jp, attempts 2–5, 7, and 9-12 were identical to attempt 1. The soft timeout likely contributed to this behavior.

Maybe asking to submit slightly different code after detecting a duplicate would help avoid this kind of issue.

### Use partial results from Permuter

Press enter or click to view image in full size

[Image](https://miro.medium.com/v2/resize:fit:1400/0*4sEYFsTuLWIlGrko.png)

Timeline when decompiling a function. Permuter ran in the background, but its output was never used in this case.

The result from the background Permuter Plugin is used in the AI-Powered Phase when it finds a perfect match.

It might be wise to feed it to Claude if it found a much better result than what Claude is working with. But we need to be cautious here, since, as explained in the 4th post by Chris Lewis, Permuter leads to introducing artifacts as signals, leading Claude to optimize around the noise, ending in a doom loop of wasteful tokens.

[4th post by Chris Lewis](https://blog.chrislewis.au/the-long-tail-of-llm-assisted-decompilation/)

### Code quality refinement step

Currently, the AI-Powered Phase asks for perfectly matching code and semantic C code. It might be too much work to do in a single phase!

We can split it into two phases: one to only find a perfect match as quickly as possible, and a second phase to refine code quality.

But it’ll likely need an alignment, since the LLM might game the system by using bad artifacts, such as asm() calls in the C code, only to bypass the system. Still, I think it’s an interesting approach worth trying.

### Support more providers

Anthropic offers powerful models for coding, but they’re not the only provider. We can even have a model tailored for matching decompilation, as Theo did a proof of concept in this post.

[in this post](https://hytopoulos.github.io/rl-decompiler/)

Mizuchi will be handy for benchmarking different providers!

### And much more

There are many ideas besides the above ones, like:

- Running the pipelines in parallel (Issue #32)

  [Issue #32](https://github.com/macabeus/mizuchi/issues/32)
- MCP Tool to read and analyze the report files (Issue #31)

  [Issue #31](https://github.com/macabeus/mizuchi/issues/31)
- Write the match to the project (Issue #5)

  [Issue #5](https://github.com/macabeus/mizuchi/issues/5)
- Better compilation errors (Issue #29)

  [Issue #29](https://github.com/macabeus/mizuchi/issues/29)

It’s a fun and endless project!

## What about…

Before concluding, let me address two more topics.

### Ralph and OpenClaw

I think I’m obliged to answer this question somewhere in this post.

There are other tools that aim to automate tasks using LLMs, such as Ralph and OpenClaw.

[Ralph](https://ghuntley.com/ralph/)

[OpenClaw](https://openclaw.ai/)

The most significant difference is that Mizuchi isn’t for AI. In the context of Mizuchi, AI is a plugin among many other plugins. Yes, it’s the most important plugin, but Mizuchi can live without it!

Also, at least for now, Mizuchi is tailored for matching decompilation, and we have a specific flow when working with matching decompilation: a clear input and a clear outcome, easy to verify and track the progress programmatically. That’s usually not the case for many LLM tasks, where the outcome is blurry.

So, in short, Mizuchi is a specific tool for a well-defined task that may use an LLM, while Ralph and OpenClaw are versatile tools powered by LLMs.

### Support for other types of decompilations?

I’m focusing on matching decompilation because that’s the challenge that nerd-sniped me.

On the other hand, there are many gaming decompilation projects that don’t have such strict constraints imposed by the “matching” part. These other reverse engineering projects try to decompile into functionally equivalent code. That’s the case for this project for Crimsonland, for example.

[this project for Crimsonland](https://banteg.xyz/posts/crimsonland/)

But what if Mizuchi supported functionally equivalent decompilation? What if it allowed us to automatically decompile code and check programmatically whether the code is equivalent to the target?

Mizuchi’s plugin architecture makes it feasible, and I’ve been planning to make it real! But that’s a topic for its own post.

## Conclusion

So, answering the question from the introduction: Yes, the data show that LLMs can work very well on matching decompilation for retro games! With a 74% average match rate across two platforms, the data suggest LLM-powered decompilation is not just viable but highly effective for the majority of functions.

Of course, there are some caveats. It’s a small sample size with aggressive timeouts. Still, as far as I know, it’s the very first benchmark evaluating LLM performance on matching decompilation using real, in-progress matching decompilation projects. I believe this data offers a baseline for further refinements to this approach. Now, it’s clearer what should be improved and, more importantly, how to measure improvement.

This result is further evidence of what other folks, like Lewis, have been showing in real projects. I hope to see even more people engaging with it!

And more importantly, at least for me, I’ll certainly use Mizuchi when I start to decompile Klonoa: Empire of Dreams! And that’s the topic for the upcoming main chapter: let’s apply the tooling and knowledge we’ve been building over the last few chapters to a new matching decompilation project.

🚶️ Short detour: benchmarking Fable 5

[🚶️ Short detour: benchmarking Fable 5](https://medium.com/p/ebb42f67c784)

➡️ Read the next chapter: Starting a Decompilation Project from Zero: Claude Code and 51% of a 2001 GBA Game

[➡️ Read the next chapter: Starting a Decompilation Project from Zero: Claude Code and 51% of a 2001 GBA Game](https://macabeus.medium.com/starting-a-decompilation-project-from-zero-claude-code-and-51-of-a-2001-gba-game-a9a75a4c1487)

Follow me on Twitter and Bluesky to stay in the loop. I’m active on the decomp.me Discord server as well, my username is trickster.42.

[Twitter](https://twitter.com/bmacabeus)

[Bluesky](https://bsky.app/profile/macabeus.bsky.social)

[decomp.me Discord server](https://discord.gg/gYwDCCWC)

I’m posting the next chapters on my Substack. See you on the next chapter there! 👋

[Substack](https://substack.com/@macabeus)

Cover image by Turbo Topology. Source

[Source](https://sketchfab.com/3d-models/detective-tails-the-murder-of-sonic-9428a4c6a131450998fc669215392be0)

Relevant xkcd: #356

[\#356](https://xkcd.com/356/)
