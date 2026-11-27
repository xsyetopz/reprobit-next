---
title: "An open letter to matching decompilation devs using AI: can we agree on some basics?"
url: "https://www.reddit.com/r/decomps/comments/1y2wca7/an_open_letter_to_matching_decompilation_devs/"
canonical: "https://www.reddit.com/r/decomps/comments/1y2wca7/an_open_letter_to_matching_decompilation_devs/"
captured_at: "2026-11-22T23:58:28.638Z"
adapter: "reddit"
scope: "thread"
site: "Reddit r/decomps"
coverage:
  "comments_reported": 16
  "comments_captured": 16
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

# An open letter to matching decompilation devs using AI: can we agree on some basics?

**Necessary\_Pool** · 83 · 2026-11-22T18:34:20.000Z · `t3_1y2wca7`

I saw someone wrote something like this 4 days ago for recomps and ports, and wanted to write an equivalent for matching decompilations. In some ways this is a vent post. I've seen a bunch of people doing stuff that I think is counterproductive to their goals and to furthering the broader community goals, and it frustrates me. I'm hoping that this letter might reach even a few people that could really benefit from it.

**Basic 1:**

Stop it with the fixed offsets, I'm begging you. This has to be number 1 because, as small of a thing as it might seem, it very evidently has the biggest impact on the code that I see in AI driven projects. Please, please, please, if you take just one thing from this letter, it should be this: put in your agents rules for matching "Use typed members, never fixed offsets. A function is not matched if it breaks this rule. In addition, a type should be declared once, and only once."

This takes your

**(bad version)**

    float r = *(float *)((char *)obj + 0x58);
    *(int *)((char *)model + 0x10) |= 1;

and turns them into

**(a lot better)**

    float r = obj->radius;
    model->flags |= MODEL_DIRTY;

More importantly, this is one of the leading causes of bad matches. It's really important that you make a hard rule against this kind of fixed offset use. **I realize this probably looks like an aesthetic issue rather than an actual problem to people not very familiar with decompilation work. I promise it's not just an aesthetic thing, that it has real impact on both how accurate and reliable the code you produce is, and almost definitely on the speed at which your AI will be able to perform decompilation work.** Good, reliable typing -> easier pattern recognition -> better, faster decomp work.

**Basic 2:**
Open source today. Get your code on GitHub. Use a very permissive license like CC0.

No one can help you if your code isn't available anywhere. You could be making things hard on yourself and not know it. There's a lot of tools out there that are immensely helpful that not everyone is aware of, for one. In addition, if you don't have a lot of programming experience, it's very possible your code is littered with "fakematches": things that TECHNICALLY bytematch but are very obviously not what the original developer intended. These really aren't things you want to fix after the fact. The earlier your code is clean and readable, the easier things will be in terms of matching and the easier it is to keep things readable.

Part of the reason that bytematching decompilations are good and worth doing is that most games share code with at least a few other games: by doing a bytematching decompilation, you're not just making progress towards source code availability for one game, you're making progress towards a broader effort for many games. When you make your project available for everyone online from the start, it means that people can begin iterating on what you've done sooner. This is another reason why good clean code is important.

Open sourcing your code now also prevents you from running into the old "but I was secretly decompiling this game first!" mixup.

**Basic 3**:

Use debug symbols when you can get them. Check [debugging.games](debugging.games) for related games that might share symbols. Anything helps: accurately named code helps with matching and dramatically improves readability. Even a little bit is a lot.

**Basic 4**:

Readability can't be a later thing.

The point of bytematching decompilation is readability, ultimately. If you don't care at all about readable code, the binary is perfectly fine and adequate. Making a complicated web of misunderstood structures and messed up vtables is really really hard, even for AI models. If you start with hard, concrete rules about code readability and about bad matches, you can save yourself a lot of work and tokens later.

If you want a very minimal framework to help keep your code clean and readable, can I recommend mine?: [https://github.com/lemiur/Game-Decompilation-Skill-For-Claude](https://github.com/lemiur/Game-Decompilation-Skill-For-Claude)

I shill it frequently not just because I want people to use my work (though that's always nice!), but because I want to spare myself having to see more decomps with poorly matched code.

**ohrajaaa** · 38 · 2026-11-22T19:19:56.000Z · `t1_19t18jdo4l128`

Bold of you to assume we read our code.

**dannipuu** · 14 · 2026-11-22T18:38:17.000Z · `t1_19t16ohxx931c`

> Readability can't be a later thing.

Can't it? It still sounds like a big improvement if the starting point is "just" renaming things or folding some definitions instead of also making the output bytes right?

> **zeroz41** · 4 · 2026-11-22T19:18:03.000Z · `t1_19t18gcuobqua`
>
> sometimes making it later is a bit of necessity imo. if you truly have no debug symbols, no knowledge of where any function belongs, and every data member is unlabeled. then how are you going to make everything clean on the first pass? it would be impossible, because you would only be able to start naming and cleaning accurately when more relations between files and callees/callers are known.

> **Necessary\_Pool** · 4 · 2026-11-22T18:45:15.000Z · `t1_19t16zonkxin9`
>
> The output bytes can be the same, but that doesn't mean the structure of the code is correct. If the structure of the code is really wrong, that will confuse an AI generally rather than make things easier. It's possible to clean stuff up, but it's way more time consuming and expensive than just having stuff be clean from the start.

> > **Garden\_Goblin\_27** · 2 · 2026-11-22T18:50:45.000Z · `t1_19t178in53ee9`
> >
> > This is what I'm learning as I'm going through a disassembly of a Genesis game that was written in C.  I'm not even looking for a matching decompilation (or even a decompilation at all, to be honest), since my goal is just functional equivalence (to start); but the amount of bullshit that leads nowhere that I'm having to clean up after my first documentation pass just going through the disassembly line by line is \*atrocious\*.  It's my fault, and now I'm committed and paying the price for it; but ugh this added another month onto the project at least, before I'm comfortable pushing it anywhere.

> > **dannipuu** · 1 · 2026-11-22T19:02:37.000Z · `t1_19t17rknsxtpc`
> >
> > Applying some simple true/false rules like the first one in your post is a no-brainer. Outside of those, I am not sure what's more efficient, because there is no good feedback loop for readability, best you can do is adversarial using different models, and that can definitely produce slop or waste a lot of tokens with nitpicks. I think "readability on-demand" is good actually

> **shinyquagsire23** · 1 · 2026-11-22T19:10:06.000Z · `t1_19t183kzb1iub`
>
> It can't, getting inheritance and structures consistent is the number 1 priority if you do it right. You can't know what an individual function is accessing if you don't know where else a structure is being written to and read, and inheritance is relational by definition.
>
> You can clean up minor inconsistencies later, but minor would be maybe a small handful of structures duplicated, not every structure duplicated every time like some of these decomps that divide and conquer per-function.

**VisceralMonkey** · 7 · 2026-11-22T20:16:03.000Z · `t1_19t1b1h97qlyc`

I’m not entirely sure what it means and that’s probably the problem. But, I will do this. Thank you.

**daedric** · 10 · 2026-11-22T18:47:10.000Z · `t1_19t172rtwhlsx`

I think Basic 1 was solved decades ago, but it's more commonly know as:

**DO NOT USE MAGIC NUMBERS**

Ex:
<https://medium.com/coding-beauty/avoid-magic-numbers-dcb7fff5784b>

> **dannipuu** · 2 · 2026-11-22T18:49:32.000Z · `t1_19t176kevv734`
>
> Naming offsets is not the same as defining the structs although they should hopefully be similar in terms of difficulty

> **eteran** · 1 · 2026-11-22T19:01:57.000Z · `t1_19t17qi6qhpcy`
>
> I think it's even more preferably described as "use structs over offset and dereferencing whenever it makes sense"

**a\_triloba** · 2 · 2026-11-22T20:09:15.000Z · `t1_19t1aqkm9tdwi`

This is appreciated. I'd like to hear from more software engineers and developers to hear what other best practices projects should follow

**BusBoatBuey** · 1 · 2026-11-22T22:54:40.000Z · `t1_19t1i46qekhpd`

A basic for me is: stay in your folder. Stop spreading across my entire computer like dandelion seeds.

**fschpp** · -1 · 2026-11-22T19:03:33.000Z · `t1_19t17t330jr8k`

I think that the hate comes from the artist that use AI as they're own art, then the general public made de extension to software developmen

**hoodwinkedfool** · -1 · 2026-11-22T19:54:00.000Z · `t1_19t1a22w5v4bm`

they can't read

**IriFlina** · -2 · 2026-11-22T19:32:09.000Z · `t1_19t192zeutcww`

You’re better off sending this to the claude code team so they can add it to their system prompt
