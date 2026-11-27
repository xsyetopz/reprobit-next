---
title: "All PS2/Xbox/Gamecube ports/decomps should easily run 720p@60FPS on most Intel630/520HD 8 Gig Potatoes..."
url: "https://www.reddit.com/r/decomps/comments/1xyxuni/all_ps2xboxgamecube_portsdecomps_should_easily/"
canonical: "https://www.reddit.com/r/decomps/comments/1xyxuni/all_ps2xboxgamecube_portsdecomps_should_easily/"
captured_at: "2026-11-18T18:20:41.170Z"
adapter: "reddit"
scope: "thread"
site: "Reddit r/decomps"
coverage:
  "comments_reported": 30
  "comments_captured": 30
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

# All PS2/Xbox/Gamecube ports/decomps should easily run 720p@60FPS on most Intel630/520HD 8 Gig Potatoes

**SufficientAttorney22** · 46 · 2026-11-17T22:10:03.000Z · `t3_1xyxuni`

Dark Watch and 007 Night Fire and Metroid Prime and Gran Turismo 3 and even Skate3 for 360 do it easily. But I noticed the majority of the rest have MAJOR stutters at the lowest resolution. I can emulate Jet Set Radio at almost a constant 60fps at 2x resolution, but the two recomps STRUGGLE hard. Midnight Club 3 and Takedawn run pretty good, but could use more optimization. There are two recomps for Need For Speed Hot Pursuit 2 for PS2, one using Go Dot and I'm glad I can cap both at an almost constant 30FPS at 2x resolution, which is cool, but I think they can do 60FPS.

The thing that sucks I guess is that I'm assuming most developers don't use intel integrated potatoes for development and testing.

**rossbalch** · 27 · 2026-11-18T01:01:38.000Z · `t1_19ssqn0osszds`

It's because static recomps often require translation layers still that eat up resources. Only true decomps can actually can fully remove all the OS, graphics API, and other overheads.

> **spriteice** · 6 · 2026-11-18T01:26:20.000Z · `t1_19ssrqoo8mf4y`
>
> The most educated comment in this thread. Realistically many recomps should only run better than emulators on your hardware with the state of the tools right now. For many Xbox recomps, the currently available tool chains still require the NV2A graphics chip, the MCPX audio chip, and the kernel to be emulated in some way. Mind you they can be extremely stripped down and targeted versions of these hardware components, but that is a game by game basis depending on which functionality the games need from these emulated layers.
>
> Another problem is that the Xbox is a single threaded machine, and the games code often assumes this, so multi threading can be very hard to implement without some kind of solid timing management, which the currently available tool chains do not really have yet and takes tens to hundreds of hours of testing to get right depending on the game.
>
> There are a few projects that work on lifting the direct 3d calls without a translation layer, but they still have fairly significant graphical errors and can often run worse than just emulating the NV2A layer.
>
> All that being said, it is reasonable to ask that a recompiled game running at your native resolution, with tweaks like draw distance, texture filtering, anti aliasing, widescreen, texture upscaling, and more, runs at the same or better performance than an emulator at this point in time.

**Logical-Database4510** · 56 · 2026-11-17T22:20:00.000Z · `t1_19ssjfhtf7sbm`

OP suddenly realizing why QA in the AAA scene costs tens of millions of dollars

**maxscipio** · 14 · 2026-11-17T22:25:38.000Z · `t1_19ssjoj0cc4kh`

running a profile can help - AI assisted recomp tend to leave out performance enhancements.

I think comparisons are useful.

The **Intel HD/UHD Graphics 630** is roughly **19 to 22 times faster** in raw compute power than the Original Xbox's [**NVIDIA NV2A**](https://www.google.com/search?ibp=oshop&prds=pvt:hg,pvo:29,mid:576462523323182066,imageDocid:9700004950525930919,gpcid:4610357549756418500,headlineOfferDocid:4375190877070145659,catalogid:5978120056296949479,productDocid:14364414277756121564&q=product&sa=X&ved=2ahUKEwiwh4W_hZCYAxVvEzQIHf7uAmEQxa4PegYIAAgPEAM) GPU.

A **7th-generation Intel Kaby Lake processor** (such as a typical quad-core laptop chip like the [Core i7-7700HQ](https://www.google.com/search?ibp=oshop&prds=pvt:hg,pvo:29,mid:576462339622638730,imageDocid:1055543427414794276,gpcid:2665376172218424540,headlineOfferDocid:9325035917213820305,catalogid:5187560774246113727,productDocid:244580829952965280,rds:PC_2665376172218424540%7CPROD_PC_2665376172218424540&q=product&sa=X&ved=2ahUKEwiegebthZCYAxUhFjQIHRwSCu4Qxa4PegYIAAgSEAE) or desktop [i5-7500](https://www.google.com/search?ibp=oshop&prds=pvt:hg,pvo:29,mid:576462916711099607,imageDocid:17908691737150088162,gpcid:17228243967317583220,headlineOfferDocid:16906317453897760394,catalogid:13849065181904624132,productDocid:9458133906336353220,rds:PC_17228243967317583220%7CPROD_PC_17228243967317583220&q=product&sa=X&ved=2ahUKEwiegebthZCYAxUhFjQIHRwSCu4Qxa4PegYIAAgSEAI)) is roughly **10 to 15 times faster per single core** and **40 to 60+ times faster in multi-core workloads**than the Original Xbox's custom [**Intel Pentium III**](https://www.google.com/search?ibp=oshop&prds=pvt:hg,pvo:29,mid:576462783180171573,imageDocid:12658768582847860217,gpcid:244397894722289003,headlineOfferDocid:7526843013359911179,catalogid:17244133427164074388,productDocid:9280309159416107551&q=product&sa=X&ved=2ahUKEwiegebthZCYAxUhFjQIHRwSCu4Qxa4PegYIAAgSEAQ) CPU.

Note: a recomp isn't a port of a decomp so some overhead is still there.

**thefarage1** · 14 · 2026-11-17T22:19:57.000Z · `t1_19ssjffba4p45`

Agreed 100%. If those games could run on what is now considered underpowered and/or outdated hardware, their decomps/recomps should be made that way as well at least on native settings.\
This would make them more enjoyable on hardware like the Steam Deck if built well.

> **thedybbuk\_** · 15 · 2026-11-17T22:28:08.000Z · `t1_19ssjsjnq5r1c`
>
> I wonder whether AI is using modern hardware specs as its baseline when translating the code, rather than older hardware.
>
> Code running natively should be considerably more efficient than emulation, yet some of these recomps are surprisingly demanding for games that were originally designed to run on consoles 15 years ago.

> > **thefarage1** · 4 · 2026-11-17T22:29:20.000Z · `t1_19ssjugznp99f`
> >
> > Yeah that’s quite possible. AI likely won’t consider potato specs if the user doesn’t tell it to.

> > **Patas644** · 0 · 2026-11-18T01:57:51.000Z · `t1_19sst5ajs0zz7`
> >
> > Por eso la mayoría son una mierda, pocos como Sonic unleashed y God of war 3 son buenos en cuanto a funcionar en hardware limitado se refiere

> **WhiteBlackBlueGreen** · 6 · 2026-11-17T23:12:48.000Z · `t1_19ssls9pnmxi9`
>
> Well its not always that simple. Just because an older game is recompiled for PC doesn’t mean it’ll run well. The original code was built around specific hardware, and modern PCs handle those operations differently. A 1:1 recompilation doesn’t magically optimize anything, so even powerful hardware can struggle with inefficient code, hardware-specific behavior, and the overhead of recreating how the original system worked.

**HANEZ** · 6 · 2026-11-17T22:33:03.000Z · `t1_19ssk0fr4x6aq`

Your right. But not every decomp is the same. There’s no standard. At least not yet.

> **dask1271** · 4 · 2026-11-17T22:59:37.000Z · `t1_19ssl73j6zspu`
>
> Totally agree. They should be tiers, because a PS2 recomp game can ask more resources than a X360 one, which is insane.

> > **Malygos\_Spellweaver** · 2 · 2026-11-18T03:11:38.000Z · `t1_19sswfrasky74`
> >
> > Remember PS3 struggling with alpha effects that the PS2 roflstomp'd thru?

**dask1271** · 6 · 2026-11-17T23:07:23.000Z · `t1_19ssljkl7lsf9`

PS2 it was, and it is, pretty hard to work for.
It's architecture is...a little different of what we have today. But certainly that's not an excuse for a recomp to ask so many resources.

Even less justifiable are the xbox and GameCube recomps. Those have an architecture almost 1:1 to the modern computer.

If a recomp ask more from a PC than an emulator and has the same features...is it really worth it?

**Capable-Sky-8995** · 5 · 2026-11-17T23:23:32.000Z · `t1_19ssm9ijk4jz7`

They should. If nothing is being emulated, you should be getting **native** performance. Which in the case of Xbox/PS2/GC should mean it can run on anything.

But most recomp devs just "optimize" for their own hardware and call it a day. If more recomp devs worked with potatoes, they would be **forced** to optimize. I wish more recomp devs actually used potatoes.

I also wish they kept in mind older devices without Vulkan support, because there is no reason why a device without shouldn't be able to run older console games just because it doesn't support the latest graphics API. OpenGL still exists and performs great on those devices.

> **arrowflask** · 2 · 2026-11-18T04:05:43.000Z · `t1_19ssyulo60kki`
>
> Yes. I notice that many recomp devs are using top-range consumer hardware, 5090s and 5080s paired with 9950X3Ds or closer tied processors. Some of them do take optimization into consideration preemptively, but most only remember that potatoes and even midrange PCs exist, when someone opens an issue for poor performance on weaker hardware.
>
> Even if these are their main development machines, I think it's funny how most of these devs don't even seem to own some other weaker hardware to do some QA and testing (or they don't care to test on their spare potatoes).

> **Patas644** · 1 · 2026-11-18T02:00:44.000Z · `t1_19sst9x49ruw2`
>
> Real, a cualquier recomp que vea que solo lo están probando con 4080s o 5070s o ryzen am5, paso.

**Yayo1990** · 4 · 2026-11-17T22:27:01.000Z · `t1_19ssjqrcmrm02`

I can tell you that Melee does run at 720p/60 FPS on PS3

**halo\_gamer923** · 4 · 2026-11-18T04:56:10.000Z · `t1_19st13m8d8ch0`

all the og xbox game recomps like from russia with love and black ran perfectly on my 8gb ram intel i3 basic laptop.  just make sure to turn off anti-aliasing and anisotropic filtering for the best performance if you are on low end hardware.

> **SufficientAttorney22** · 2 · 2026-11-18T05:22:50.000Z · `t1_19st2afwdft41`
>
> I ALWAYS do that first.

**Jadentheman** · 6 · 2026-11-18T03:44:34.000Z · `t1_19ssxwniwh8o3`

Because most the authors don't actually know how to code, just ask the AI to do something.

**Dora\_De\_Destroya** · 3 · 2026-11-17T22:18:07.000Z · `t1_19ssjcgiu5kps`

Steam deck and Steam Machine are my baseline go tos since they have pretty average hardware

**dregomz** · 3 · 2026-11-17T22:57:59.000Z · `t1_19ssl4hnamvc1`

They should but they won't as most decomps are tested on only single piece of hardware.

**mehtehteh** · 3 · 2026-11-18T01:46:52.000Z · `t1_19sssno4vnyn8`

This is the early wild west days of recomps via AI. At some point it will get better or actual coders will get around to making better versions at some point. Right now its take what you can get

Also holy crap. Intel 630 goes back to Intel 8000 CPUs in 2017. No developers are not building on 10 year old PCs.

**Flashpower45** · 3 · 2026-11-18T12:35:43.000Z · `t1_19stllirnr0gw`

Korrekt. Die meisten kriegen gar nicht mit wie schlecht ein Recomp ist mit ihren RTX Grafikkarten. Beispiel Mario Kart GP. Wenn die Shader erstmal kompiliert sind funktioniert es in Dolphin wie es soll. Der Recomp ist vorsichtig ausgedrückt nicht so gut 😁. Zum Thema Optimierung: Der Supermodel Emulator zieht mit OpenGL 2.1 auf full HD 1980x1080@60hz auf meinem Laptop mit i5 6300HQ und GTX 950m perfekt ab ! Da sieht man das die GTX 950m jawohl die entsprechende Leistung hat.

**Disgruntled-Cacti** · 2 · 2026-11-17T23:03:20.000Z · `t1_19ssld2bvwxsx`

There’s a difference between vibe coded software and software that at least has some sort of human expert in the loop and I’ll leave it at that.

**\-Trash--panda-** · 2 · 2026-11-18T00:14:28.000Z · `t1_19ssojaovh0yt`

I have a computer that is a bit more powerful than yours using a early ryzen CPU/iGPU. I have found ps1 recomps that were so poor that they stuttered on my computer. If they had some massive enhancements that would be something, but it was mostly just the game upscaled.

**waterclaws6** · 2 · 2026-11-18T06:17:54.000Z · `t1_19st4quzedput`

This is true. However, a lot of devs only optimize on the top-end, unless they either have a pc handheld or actually work on lower-end hardware.

**Windows\_\_94** · 2 · 2026-11-18T11:44:19.000Z · `t1_19stjazjkrzts`

From what ive personally seen, recomps end up with a ton of obfuscated functions like FUN_268B4C1 for example with no clear indication of what they really do or their intended purpose was, they read close to assembly more than anything.

E.g. xbox 360 usually shows me around 60k of these.

But once you spend time profiling and hammering batches of functions with ghidra & various other tools to work out their possible names, variables, enums, callers etc you start to uncover what they originally where.

I start on leaf functions and gradually work my way up the call graph with this methodology. Eventually I start to have a series of human readable code that im somewhat confident is what it originally was.

From here I take the ship of Theseus approach, I create a build that can swap between using the original garbled functions and what im assuming is the ungarbled functions Ive just worked out using launch flags. After some rounds of testing, if everything is behaving as normal I note it as a possible success and then move on to the next batch.

With this ive been able to get Fable 2 from <10fps on Steamdeck to ~25fps in bowerstone. Through identifying aspects where the 3d engine was needlessly waiting on the gpu or inefficiently checking, reading and writing to memory rather than finding repeated calls with awaits that could be cached.

Some issues still remain and no real optimisations have been made yet but the performance improvements are real and I agree potato hardware (aslong as it matches or exceeds the consoles hardware) can/should be able to run these after the decomp is given some TLC.

My experience is a Java developer with some 68K knowledge moving into system level Rust/C++ as an evening hobby.

Edit: include mention of launch flags for swapping, not hot-swapping, between garbled and ungarbled functions for testing and assertions.

**stellarola** · 2 · 2026-11-18T15:05:11.000Z · `t1_19sts9ibqsxer`

Average GPU for this subreddit - RTX 5070/5080

Me - RTX 3060 ti

**CodyRhodesBurner** · 1 · 2026-11-18T16:52:16.000Z · `t1_19stx1g4urnl0`

I do my best to optimize my recomps, but at the same time aren’t these all open source on GitHub?

I never understood complaining about it when you could just do it yourself?
