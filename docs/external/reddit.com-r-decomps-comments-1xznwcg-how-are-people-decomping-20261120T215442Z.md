---
title: "How are people decomping?"
url: "https://www.reddit.com/r/decomps/comments/1xznwcg/how_are_people_decomping/"
canonical: "https://www.reddit.com/r/decomps/comments/1xznwcg/how_are_people_decomping/"
captured_at: "2026-11-20T21:54:42.410Z"
adapter: "reddit"
scope: "thread"
site: "Reddit r/decomps"
coverage:
  "comments_reported": 47
  "comments_captured": 47
  "comments_deleted_or_removed": 0
  "pending_comment_ids": 0
  "unresolved_more_nodes": 0
  "requested_comments_unavailable": 0
  "more_requests": 0
  "cached": false
  "sort": "confidence"
  "partial": false
content_trust: "untrusted"
---

# How are people decomping?

**Archolm** · 35 · 2026-11-18T19:11:00.000Z · `t3_1xznwcg`

Just an honest question, how are you all doing these mega games while I have been trying to get a clean, good build of Geometry Wars: Galaxies on the Wii working?

I'm using Claude coding and i'm at session 48 but while it does show progress its not nearly going fast enough to my liking, or the idea I have of porting over a 15ish year old Wii game.

I'm using the Mario 64 method of supplying the iso while the code gets run natively on PC. (Linux even) with the idea to port it over to Android.

So if anyone has suggestions or useful advice, please do tell!

Claude Pro by the way, Sonnet 5.5.

**strutb** · 42 · 2026-11-18T20:12:41.000Z · `t1_19su5z9bbiesl`

A bit-exact decomp is insanely difficult. It’s up there with the hardest type of engineering work there is, and frontier AI models don’t change that (they just do it faster).

For an example: I’ve been working on a bit-exact decomp of a PSP game where I know the original language, have the exact compiler, and sometimes don’t achieve more than a few kilobytes in a day when the full original source is a couple megabytes. That’s with cutting edge AI models doing the work at my direction.

So….it’s just hard.

Static recompilations, on the other hand, are far simpler. They aren’t going back to original source at all, they’re taking the binary compiled for one architecture and converting it for a new architecture, relying on shims (“emulation”) for all the gaps and pieces it doesn’t understand. It doesn’t know how the code works, it just makes it run.

> **arthor** · 3 · 2026-11-19T02:26:51.000Z · `t1_19sumo20srlnm`
>
> any repos for guiding an LLM for this? also what models are people using?

> **Frogacuda** · 2 · 2026-11-19T13:11:41.000Z · `t1_19svffh8aihd0`
>
> There is the in-between, decomp to pseudo code and functional rewrite. That's also quite intensive, and it introduces a whole new set of challenges because you need to be really rigorous about validating and matching every routine to make sure it doesn't introduce new bugs. Even if you do it right, sometimes the AI will go rogue and go "oopsie poosie" when called out. But in the end, you can still produce good, clean, understandable source code that can form the basis of ports, mods, remasters, and updates in ways that static recomp can't.

**TsuriThugga** · 19 · 2026-11-18T20:01:28.000Z · `t1_19su5h9mz225h`

Recomp and decomp are two different things, decomps takes weeks/months even with AI, a vanilla recomp hours/days.

**Aryuto** · 8 · 2026-11-19T02:06:51.000Z · `t1_19sulrxq841ut`

To be clear. A matching decompilation with good quality is generally (rough estimates) a 5-10 year project for a solo human, 3-5 years with a small group, or a few months with ai. Decomps are a huge project that involve complete understanding and naming of every function and variable in the game, and often some amount of replacing hardcoded offsets/references with function calls and the like so it can be safely expanded.

A recomp is maybe a few hours of ai making a game somewhat playable on pc, then probably a few days or maybe weeks to make it polished with new features.

You gotta decide on one or the other very early. You can decomp and then recomp for sure, but 99% of what gets posted here is just a recomp.

**Realistic-Snow-3532** · 5 · 2026-11-18T19:44:28.000Z · `t1_19su4py27ei5f`

For me freebuff on the 8$ plan using deep seek 4.1 flash fast has been working great,  I may use Claude to figure out the tool chain/ compiler and then switch over, I had it write a program to auto prompt it "continue " every 10 minutes and I fill the queue with about 20 continues and it runs all day and all night 24/7 I'm doing a psp monhun game right now

> **Archolm** · 3 · 2026-11-18T19:48:00.000Z · `t1_19su4vn70ypnn`
>
> The auto continue seems nice, to optimize the downtime when working/sleeping. Cheers!

> > **Realistic-Snow-3532** · 3 · 2026-11-18T19:49:41.000Z · `t1_19su4ycbj304k`
> >
> > I'm honestly unsure how it never hits my usage limit lol

> **F\_Kal** · 1 · 2026-11-19T09:56:18.000Z · `t1_19sv6pqphxfmq`
>
> I have a similar setup (using the free tier deepseek 4.1 flash medium of opencode provided models), and I’m trying to decomp a 15,000 functions program: i have the ugly ghidra pseudocode and the symbol names and I am trying to reconstruct the human readable code. Yet the progress is gruesomely slow to the point that I was wondering whether this AI model is fit for the purpose - for some reason i can’t do more than 4-5 functions at a time - after weeks, i am stil at 800 out 15000  (average 5-10kB per batch) - using ghidra and REA - any tips/suggestions?

> > **Realistic-Snow-3532** · 2 · 2026-11-19T12:30:18.000Z · `t1_19svdl0tg0oyq`
> >
> > What's your agent file look like? I feel like workflow is important for these kind of things.

> > > **F\_Kal** · 1 · 2026-11-19T13:42:43.000Z · `t1_19svgtaqdotwk`
> > >
> > > this would be it, after the latest iteration - I had codex externally audit the method and improve it, though it still sucks!
> > >
> > >     ***** **** reconstruction - agent guide
> > >
> > >     This repository reconstructs ******* v**.** (macOS x86_64) as readable C++20 under recon/. It is not a game or byte-matching decompilation: fidelity is behavioural, proven by executing shipped code and replaying results. Skills in .opencode/skills/ (matcher, wall-identifier, helper-recovery, permute, decomp-code-rules, orchestrator, native-platform-porting) define the method; this file is the project adapter.
> > >
> > >     Read this first. It overrides skill defaults.
> > >
> > >     Authority and evidence
> > >
> > >     Sources of truth, strongest first:
> > >
> > >     1. Shipped binary - /Applications/*****.app/Contents/MacOS/***** (..**), mirrored at analysis/slices/*****.x86_64; sha256 in analysis/README.md. Disassembly, exports (analysis/exports.csv), RTTI, strings and decompiler data derive from it.
> > >     2. In-process oracles - real functions called on shipped code, results replayed bit-for-bit (analysis/*-oracle.txt). Only proof of scalar/value/framing contracts.
> > >     3. Ghidra pseudocode - pseudocode/ and analysis/ghidra/. Useful for structure/constants; SSE aliasing, aggregate copies and ABI require disassembly.
> > >     4. Inference - permitted for names, comments and architecture, always marked.
> > >
> > >     Non-obvious claims in recon/ carry markers:
> > >
> > >     * // GHIDRA:0x<addr> or // 0x<addr> maps functions to shipped code; the state builder reads these.
> > >     * // UNVERIFIED marks unproven layout gaps, names or services.
> > >     * Header comments cite pseudocode, JSON writer addresses and oracle sections.
> > >
> > >     Never invent offsets, sizes, signatures, fields or APIs. Fields cannot extend past the next observed offset; unobserved bytes are padding, not invented members.
> > >
> > >     Fidelity
> > >
> > >     * Tier A is complete when defined, compiled by tools/check-recon.sh, and checked by passing oracle replay or documented disassembly-level reading where no oracle exists. “Looks equivalent” is insufficient.
> > >     * Byte identity is not a goal; no fakematch rules. Idiomatic C++ (enums, std::optional, std::clamp, range loops) is allowed when behaviour/layout are proven.
> > >     * Evidence wins. Style must not contradict proven offsets, widths, signedness, evaluation order or side effects. When readability conflicts, preserve evidence and comment.
> > >     * Verification priority: (1) compilation; (2) passing oracle or documented disassembly reading; (3) structural comparison (objdump, tools/class-bundle.py); (4) byte matching (tools/byte-match.py) as secondary triage. SHAPE/DIFF do not prove bugs; BYTE does not prove correctness.
> > >     * Compiler-codegen matching is optional. Once compiled and behaviourally verified, stop pursuing byte identity unless requested or a discrepancy raises a behavioural question.
> > >     * Speculative codegen experiments (flags or source rewrites solely to reproduce instructions) are limited to two attempts per discrepancy. Then record difference, evidence, confidence and attempts. Revisit only with new evidence or user instruction.
> > >     * Unresolved differences go in class/function headers: shipped address, divergence, evidence of correct behaviour and confidence (high/medium/low). Require disassembly-level reading with offsets, not “looks equivalent”.
> > >     * Assembly similarity (BYTE, SHAPE, SAME-OPS) never establishes equivalence. Require execution or documented reading of every branch, store and side effect.
> > >     * Preserve and label original bugs/quirks (e.g. Set****** clamping NaN to **** using ****/****).
> > >
> > >     Two tracks
> > >
> > >     * Tier A (recon/src/<module>/<Class>.{h,cpp}): readable, hand-written, layout-asserted, tested. Only these count.
> > >     * Tier B (recon/src/generated/, via tools/pseudo2cpp.py): mechanical, compile-only, never counted as reconstructed. Fix the emitter, not generated files.
> > >     * recon/src/gui/ is presentation scaffolding, not evidence.
> > >     * Batch size follows evidence payload, not function count. Related functions share packets; parsers/serializers/localizers get separate batches. See PROCESS.md (“Batch selection”).
> > >
> > >     Project adapter
> > >
> > >     verbthis project
> > >     buildtools/build-recon.sh (library), tools/build-gui.sh (demo)
> > >     match <unit>compile TU (tools/check-recon.sh) + oracle replay; byte-match optional
> > >     diff <function>objdump -d slice vs reconstruction or oracle dump
> > >     reportpython3 tools/project-state.py status / next --brief
> > >     gatetools/run-tests.sh (12 gates)
> > >     disasmobjdump -d --start-address=... analysis/slices/*****.x86_64
> > >     decompile <fn>Ghidra DecompileOne.java (see Commands)
> > >     xrefanalysis/state/project-state.json (callers, callees), exports.csv, nm
> > >     classanalysis/layouts/layouts.json + recon/src/<module>/<Class>.h
> > >     permuteextend oracle input table (no permuter)
> > >     batchlargest ready evidence packet: tools/class-bundle.py --addr/--addrs/--addr-file
> > >     closeouttools/closeout.sh (all gates once; analysis/state/closeout.json)
> > >     evidencetools/evidence-log.py ledger (deferred/compiled/verified)
> > >     rulestools/check-recon.sh + naming/evidence rules
> > >
> > >     Coverage belongs in machine-readable state, not prose:
> > >
> > >     * analysis/state/project-state.json: function addresses, classes, sizes, Tier A source, layouts, offsets, callers/callees, signatures, confidence, validation.
> > >     * analysis/state/queue.json: dependency-aware batches.
> > >     * analysis/state/summary.md: human-readable snapshot.
> > >
> > >     Code rules
> > >
> > >     * C++20, clang++, macOS libc++; gates use -Wall -Wextra -Werror.
> > >     * Four-space indentation, preferably 80 columns, #pragma once; class header first, then standard/project headers.
> > >     * Classes T**** / T*** / T*** as mangled; members m_PascalCase, methods PascalCase, locals lowerCamelCase. Names derive from symbols, JSON keys, assertions or strings; invented names marked in headers.
> > >     * Public members for directly accessed flag/value structs without exported accessors; otherwise private with methods.
> > >     * Proven widths only: std::int32_t, std::uint8_t, bool, double, float, libc++ containers/optionals. Never guess long.
> > >     * Assertions throw ****Exception via ****Assert / *****Assert2, preserving original expression, embedded file path and line. Expression text is never formatted.
> > >     * Unfinished functions remain in generated Tier B, never half-finished Tier A.
> > >
> > >     Layout and tests
> > >
> > >     * Layouts use static_assert(offsetof(...)) and sizeof inside a ...LayoutProbe friend struct in .cpp.
> > >     * Value classes get tools/oracle/<name>_oracle.c, a script injecting a dylib into a re-signed app copy, and replay tests in recon/tests/. Hex/%a dumps permit bit-exact comparisons.
> > >     * Add Tier A sources via python3 tools/gen-build-lists.py, rebuild state and run all gates.
> > >
> > >     Commands
> > >
> > >     # gates: required before declaring a round complete
> > >     tools/closeout.sh  # --skip-gui / --from <step>; writes closeout.json
> > >     # state
> > >     python3 tools/project-state.py build  # ~15 s; recon/ + pseudocode/
> > >     python3 tools/project-state.py status
> > >     python3 tools/project-state.py next -n 3 --brief
> > >     # bulk workflow: one batch at a time
> > >     python3 tools/class-bundle.py <Class> -o work/bundle-<class>.md
> > >     python3 tools/class-bundle.py --addr-file work/batch-addrs.txt -o work/packet-<batch>.md
> > >     python3 tools/evidence-log.py work/evidence/<batch>.json add verified 0xADDR \
> > >         --kind oracle --ref tools/check-<class>-oracle.sh
> > >     python3 tools/evidence-log.py work/evidence/<batch>.json report
> > >     tools/ledger.py work/round-<class>.ledger
> > >     python3 tools/byte-match.py --src recon/src/<mod>/<Class>.cpp --class <Class>
> > >     python3 tools/byte-match.py --all
> > >     # one function
> > >     /Applications/Ghidra.app/Contents/Resources/ghidra_12.1.4_PUBLIC/support/analyzeHeadless \
> > >         analysis/ghidra ***** -process *****.x86_64 -noanalysis \
> > >         -scriptPath tools/ghidra -postScript DecompileOne.java <out.c> <0xADDR> 300
> > >
> > >     Repository hygiene
> > >
> > >     * Git enabled. Commit each completed class with function count and gate result. Regenerable/heavy artefacts are ignored (.gitignore): work/, original/, analysis/slices/, analysis/ghidra/, analysis/state/, build output. Before risky rewrites, copy files to work/; Git protects only committed content.
> > >     * Generated files belong in recon/src/generated/; analysis in analysis/; scratch in work/; round notes in notes/.
> > >     * TODO.md tracks blockers; notes/ contains dated rounds and optimization lists. Documentation describes current contracts, not diaries.

> > > > **Realistic-Snow-3532** · 2 · 2026-11-19T14:09:47.000Z · `t1_19svi0rhpcrc2`
> > > >
> > > > So what I've been doing is keeping my agent file pretty short other than the important things right what are we doin, how are we doing it and what not to do but I also add a document directory with every document we keep and update, how we update it , why we updated and when we updated so that way the agent can read the docs when it needs it and it's also not just making a bunch of docs that never get read again, I'll upload mine later.

**DrEggman399** · 4 · 2026-11-18T21:08:48.000Z · `t1_19su8hd06rgir`

Using the decompdecomp framework with various APIs and harnesses (OpenRouter, Antigravity). Decompdecomp aims to provide a byte matching decomp. Progress can be fast at times but in general it takes much longer than you expect. Have been working for a few weeks now on an N64 decompilation.

> **mr\_christer** · 2 · 2026-11-18T21:27:45.000Z · `t1_19su9bsdpt9ib`
>
> Interesting! I thought n64 decomp is very straightforward these days

> **maleficientme** · 1 · 2026-11-19T18:26:55.000Z · `t1_19svthmviub8i`
>
> Is there any reddit sub i can request for a specific game port to the PC or other platform?

**dragonfax** · 6 · 2026-11-18T23:50:48.000Z · `t1_19sufphpmectf`

Most people who say they have a decomp or are posting a "decomp" here, they're not actually decompiling the binary to readable C source code. That's a huge project that can take months and years. Instead, they're just decompiling it to an assembly-like C, which is unreadable but serves the purposes of targeting a different platform.

This is how they get a game written for the Switch on an ARM platform to run on an Intel platform like Windows.

I've done full decompilation of old retro stuff, and that can take an agent weeks to get to 90% readable C source. Weeks of heavy agent usage AND heavy compute. But something like this is just for educational purposes.

There are very few actual full decomps (readable code) available on the internet. And most of them are pre-LLM work.

**ethaseagull** · 4 · 2026-11-18T20:18:01.000Z · `t1_19su67thvivdt`

decompilation isn’t quick, even with ai

> **Archolm** · 3 · 2026-11-18T20:20:24.000Z · `t1_19su6bnv3agrp`
>
> I'm beginning to see that lol.

> > **ethaseagull** · 2 · 2026-11-18T20:23:03.000Z · `t1_19su6fx7nvrz9`
> >
> > if you really want progress, your best bet is to upgrade and set up parallel sessions with multiple models on different functions. or, you can use an app like decompdecomp to orchestrate agents for you

> **alvinchimp** · 3 · 2026-11-18T23:26:59.000Z · `t1_19suen8z43338`
>
> This, before AI it was many years with teams of people working at it.  It's still decently slow but AI has massively sped up the process.

> **lordlestar** · 2 · 2026-11-19T04:25:05.000Z · `t1_19surxx8yhnc4`
>
> decomps have become quick, before AI a team could take years to make a decomp,  now a solo dev can take 3-5 months, in the scale of things, that is quick

> > **ethaseagull** · 1 · 2026-11-19T17:08:59.000Z · `t1_19svq0hpgczfk`
> >
> > yes but not as quick as op expectedb

**WhopplerPlopper** · 5 · 2026-11-18T19:59:34.000Z · `t1_19su5e7bq8hur`

Protip: most of those projects aren't decomps.
Maybe yours isnt either...

If you instructed your AI to make a port, you're not making a decomp.

If you instructed it to decompile it, you will get that result - and it will take a long time.

Research the difference between decomp, recomp and port

> **Frogacuda** · 2 · 2026-11-19T13:04:44.000Z · `t1_19svf4biwuyjn`
>
> Even in the world of true decomps, there's a wide range from "psuedo code with translation layers/VM strapped to it" to "modernized and commented reconstruction of functionally identical code," to  "true byte-matched source code"

> > **WhopplerPlopper** · 1 · 2026-11-19T13:30:37.000Z · `t1_19svg9vfdaesg`
> >
> > You are conflating decomps and ports.
> >
> > If there's a translation later, you're talking about a step after decompilation

> > > **Frogacuda** · 2 · 2026-11-19T13:48:03.000Z · `t1_19svh1vi736g0`
> > >
> > > I'm talking about three different kinds of decomps, p-code, byte-matched, and functional reconstruction. The vast majority of people doing this are interested in porting so, I am acknowledging that even with p-code you can do that, but that doesn't mean it isn't a decomp.  All three are involve decompilation or disassembly, but they're different degrees of thorough.

> > > > **WhopplerPlopper** · 1 · 2026-11-19T16:06:31.000Z · `t1_19svn87eflu1v`
> > > >
> > > > Sure, but when we are saying "decomp" in the community, I feel pretty much everyone (as in the actual developers doing this stuff) are talking about byte matched decomp no?
> > > >
> > > > Like ignoring the casuals who call every console game playing on PC a "decomp" lol.

> > > > > **Frogacuda** · 2 · 2026-11-19T16:50:33.000Z · `t1_19svp6wi0wsx1`
> > > > >
> > > > > That's silly, there are many legitimate decomps that are function-for-function but not byte-for-byte and always have been. Both are legitimate paths to clean, understandable source code.

**kalek\_\_** · 2 · 2026-11-18T19:44:34.000Z · `t1_19su4q4254i8w`

I've had a lot of success getting Claude to run the game in an emulator with a debugger and comparing it to how far it gets in the recomp, make fixes, repeat. Have it make skill(s) for doing this so it doesn't repeat figuring out things like how to use the debugger or make the comparison.

It may even be good to give it the source to the emulator (in this case, Dolphin) and allow it to make custom builds so that it can make debugging modifications for better understanding.

You might find more success with Opus. I haven't personally found Fable to be that helpful over Opus though.

> **the\_vico** · 3 · 2026-11-19T04:48:55.000Z · `t1_19sut06lmnhia`
>
> So REA is a thing already in the past?

> **Archolm** · 1 · 2026-11-18T19:49:36.000Z · `t1_19su4y729iyxs`
>
> I'm not really up on the different models. What has helped me is giving GPT instructions to pass to Claude, it kept making assumptions in the codebase that just made me think the project was farther along the line than it actually was.
>
> Do you know if Opus is heavier on tokens?

> > **jptiger0** · 1 · 2026-11-18T20:36:54.000Z · `t1_19su725bwr90j`
> >
> > Yup. And Fable way heavier than opus. Haven't done any de/recomps but generally for vibecoding stuff it's a decent idea to generate a detailed plan with Opus and execute with Sonnet. Depending on your setup I think you can explicitly tell Opus to subcontract out to a Sonnet agent for rote tasks

**PhilGood\_** · 1 · 2026-11-18T20:33:21.000Z · `t1_19su6wglmcrxh`

It took me about a month and quite a few resets with claude/codex to decomp rising lands (1997) but I got to a stage i like so far.
It used ghydra to check the assembly and then I rewrote it using rust dor the game engine

> **Archolm** · 2 · 2026-11-18T20:54:52.000Z · `t1_19su7uzor8wo1`
>
> I know a tiny amount of C# so thats my goto.
>
> Appreciate the input! 👍🏻

**JohnTheRedeemer** · 1 · 2026-11-18T23:13:46.000Z · `t1_19sue20qe7m8x`

Oh man, I'm on month 2 of decompiling and converting Jade Cocoon.

Converting it to C was fine, long but arduous, making it properly PC compatible without hacks and shortcuts has been more complex, a lot for hand holding and giving proper examples.

Still in the beginning sections, but I think it's going better now. Getting the tooling up so I can compare to retail and ensure it's doing a good job side by side has been a hard part, as is separating and converting the assets into PC-native versions.

I could phone it in and go faster, but this is more of a personal desire and I'd like it to be something solid when it's done

> **themysidianlegend** · 3 · 2026-11-20T04:17:17.000Z · `t1_19swjtkyrxvtw`
>
> Cool game choice!

**lordlestar** · 1 · 2026-11-19T04:17:13.000Z · `t1_19surl9x3263m`

skill issue

A decomp usually rely on tools like ghidra/ida pro to decompile a game into pseudo c, that offloads a lot of work, Claude just do the work of turning that pseudo c into real c/cpp. If a game uses a known game engine then is easier and faster because you can reuse a lot of code.

but most decomps today are actually static recompilations, aka recomps, they use tools that translate machine code from one machine to another ahead of time, like an emulator, but the translation is done one time and the resulting code can be compiled to a ready to use executable. This method is faster than a decomp because it can be fully automated, let a alone a matching decomp like mario 64 decomp that should compile an exact binary of the original rom, that is even harder and time consuming even with ai

> **themysidianlegend** · 1 · 2026-11-20T04:26:56.000Z · `t1_19swk937j0pyd`
>
> Thanks for the info. So is bloodborne on PC a recomp or decomp?

> > **lordlestar** · 2 · 2026-11-20T04:35:10.000Z · `t1_19swkmb9i4db6`
> >
> > A recomp

**Jedeyesniv** · 1 · 2026-11-19T10:02:40.000Z · `t1_19sv6zyheqfdv`

I've been using Claude for older games and I'm finding more success with doing a kind of very enhanced custom emulation. The goal for me is to make the games super playable. It's not like I'm coding anyway, so understanding the code is meaningless to me if the game plays well. Having a lot of success with it.

**Professional-Exam156** · 1 · 2026-11-19T10:24:15.000Z · `t1_19sv7ymnmxmjo`

I built my own tools and workflows

Project <https://github.com/raulcorreia7/bof3-decomp>
Status: <https://raulcorreia7.github.io/bof3-status/>

Here is a generic skill:
<https://github.com/raulcorreia7/re-skills>

You can probably reuse this (in linux) to reverse almost anything.
I'm focusing on a ps1 game

The bit exact decomps are hard because you need to find exact libraries, compilers, file formats, proper diffing

Most of functions are trivial or a reimplemntation is fine

But byte matching is really really really hard

**Frogacuda** · 1 · 2026-11-19T12:17:57.000Z · `t1_19svd17apx3s2`

There's a pretty big range of methods that require vastly different amounts of work, so it's important to keep in mind what you're really trying to do and how much effort you want to put in.

If you're just trying to make a game work and run nice on another system with some platform adaptations, decompilation isn't the only way to do that. Static recompilation can basically convert a game to native machine code on another system very directly and then the AI can modify it for you from there. This method is kind of one step up from emulation, and it's not great if you want to make major changes under the hood, but it's perfectly viable if you want to tweak around the edges or build on top of a foundation.

Decompilation is when you want to actually get original code that you can modify and port, and here too there are degrees.

You want to start with Ghidra, if you're working from a supported platform (you are). This will decompile the game to pseudo-code, which is functional code that is hard for a human to understand but since you're working with AI, that's not super important.

Where you go from there is up to you. You can try to reverse the coder to byte matched identical code and get true source code for the source platform, like Render96. This is not necessary or even especially useful to the goal of adapting a game to a new platform, but it is the purest form of recreating it.

Then there's functional decompilation, where you rebuild the game by rewriting code that does the same thing, but isn't necessarily identical. This is less work in some ways, but also more work in others, and it's much more prone to errors and introducing new bugs, so it's really important to make sure you're doing really rigorous trace and validation checks at every step along the way. This can sometimes produce more logical, legible code and it can be better if you're trying to modernize the game as something platform agnostic, portable, and commented, but it's also easy to get caught with the AI being lazy, guessing, or taking shortcuts it shouldn't and you end up with a buggy mess.

**spriteice** · 1 · 2026-11-19T12:21:14.000Z · `t1_19svd6gsadkdc`

Recomps and decomps are different, I am assuming you are working on a recomp. A decomp gives you the full source code to do what you will with. As others have said, it takes years, or months if you are properly babysitting the AI. Recomps work from the other end, with the fully interpreted machine code. You can still achieve a lot with this, but not as much as a pure decomp due to the requirement of translation layers (emulating specific parts of the original console for your game only).

As far as how to use AI effectively, I have a few tips:

1. You should use Claude Code projects, as it tracks all the different threads for you and keeps a shared context.
2. Opus with extra effort has given me the best results so far. Fable is good for planning large code base refactors, or planning complex features (adding a vulkan renderer), but Opus is the most cost effective way to get the code from these plans created by fable.
3. Make sure your underlying tool chain is solid. This seems to be the most mature recompiler at this point: <https://github.com/BlackLineInteractive/NWiiRecomp>. Cross check between recompilers.
4. Create a solid rule set and workflow. "Do not commit game files", "find the root cause in the correct layer of code", "do not patch a symptom, find the cause" are some rules that have helped me achieve some solid results. Run through best practices, token savings, time saving, code robustness, code accuracy, etc. with an agent until you can make a rule set and workflow that agents wont get caught up on, and will create the best product possible given the constraints. It will probably take some trial and error while you can gather what works and what doesn't, and look back and adjust things from that.
5. Testing and logs. You must test things thoroughly, and part of the workflow should be test lists that you complete and send back with logs. I have interactive HTML files delivered to my PC that I fill out to send back. If you are not testing effectively, you will not get good results. Make sure the project logs as much as it can, but make sure that you can toggle them so they do not slow the game down or muddy results. Test on a variety of configurations (if possible), recompilers can pass maths off to the CPU and AMD chips can work out floating point values slightly differently to Intel, which can cause strange bugs with physics or maths on certain systems.
6. Patience and perseverance. If something is not working, or you are running into the same bug presented a million different ways, ask an agent what is going wrong, are our rules allowing this to happen, and how can we get to the real cause and avoid this issue in the future. It will take time effort, revisions to the rules and plan, and it will not always be fun changes to the code. Stick with it and be open to changing how things operate if needed.

**Connect\_Quit\_1293** · 1 · 2026-11-20T15:58:54.000Z · `t1_19sxf45tzvkv5`

Claude Pro is fine if you have some token management skills. If not you might need Max.

I wouldnt use Sonnet for reverse engineering. I tried, it was miserable. Go back to Opus 5.5

And really it comes down to the one thing most AI haters refuse to acknowledge - Even with AI its not THAT simple, it still takes quite a bit of work and effort.  Complexity will vary greatly by game, system, and your own developing experience.  Someone below said skill issue which is meaner way to put it, but there is some truth to it.

**PhoenixTineldyer** · 1 · 2026-11-18T19:21:28.000Z · `t1_19su3p0ourxwk`

Claude.

**Otherwise-Clue-1997** · 0 · 2026-11-18T21:58:03.000Z · `t1_19suaog0eri3p`

Maybe codex or REA can help

**MarcCouillard** · 0 · 2026-11-19T13:53:00.000Z · `t1_19svh9tou1ilg`

yeah Pro is gonna take forever bro...you really need to be able to have multiple chats going at once and not have that 5 hr timeout, so you NEED the top tier yearly subscription...the expensive one lol, if you want to make faster progress

edit: also the model matters, sonnet is great for writing, while Opus or Fable may be better equipped to deal with layer translations and programming code...play around with the different models to find the one that works best for you (hint: it isn't sonnet 5.5)
