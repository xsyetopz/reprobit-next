---
title: "An open letter to recomp/decomp devs: can we agree on a few basics?"
url: "https://www.reddit.com/r/decomps/comments/1xyz4xo/an_open_letter_to_recompdecomp_devs_can_we_agree/"
canonical: "https://www.reddit.com/r/decomps/comments/1xyz4xo/an_open_letter_to_recompdecomp_devs_can_we_agree/"
captured_at: "2026-11-18T12:26:17.604Z"
adapter: "reddit"
scope: "thread"
site: "Reddit r/decomps"
coverage:
  "comments_reported": 164
  "comments_captured": 164
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

# An open letter to recomp/decomp devs: can we agree on a few basics?

**Xarishark** · 877 · 2026-11-17T23:03:39.000Z · `t3_1xyz4xo`

Hey everyone,

I’m the lead dev of RecompLauncher.

First I want to say that I love where we’ve arrived as a community. In my wildest dreams I never thought we’d be playing so many exclusive games natively on our PCs, in the best versions they’ve ever been. So before anything else, thank you all for what you’ve given the community ❤️❤️❤️

Every day I’m working to support as many of your projects as I can, and I try to send changes that make them better both for the devs and the players.

**We’ve reached a point where the recomp/decomp scene needs some shared standards, so everyone has the same base to work from. Right now it’s a bit of a wild west, and players and devs both pay for it.**

**Here’s what I’m asking:**

**1. Ship one executable.**

The most important of all by far! I keep seeing projects ship a builder exe that then builds the game exe. Please stop doing this. I get it: you can’t ship the final exe with the game code inside it. But your launcher can still detect that the game is already built in a folder and drop the player straight into it on the next launch or even provide a simple play button that is not tied to another binary. [LostOdysseyRecomp](https://github.com/freefrank/LostOdysseyRecomp)does exactly this. It works perfectly, and I wish more projects copied it.

**2. Make portable the default.**

Do all your work in the folder your executable lives in. For AppImages, let us pass one directory and keep everything inside it. You’re not an installed app with an MSI and a proper uninstaller, so please don’t scatter files across the user’s home folder and other places only you know about. When someone deletes your folder, the game should be gone.

**3. Ship a tarball next to your AppImage.**

AppImages are great and you should keep shipping them. But most people playing your projects are on recent systems, and a plain tarball lets launchers like ours deploy your game even from inside the Flatpak sandbox instead of doing hacks like extracting appimages.

**4. Add a CLI flag for everything.**

Fullscreen, ROM path, build folder, all of it. It costs almost nothing in code and helps everyone. I know a lot of you put real work into your launchers. But when every game has its own launcher with its own quirks, a player who wants to try a few games has to learn each one. With flags, we can get your players into the game as fast as possible.

**5. Please consider the penguin🐧!**

Some of you know me as a Bazzite dev so I have to ask for this. Native Linux builds go a long way, and the Linux gaming crowd will love you for it. Do us a favor and where possible release native linux builds. Its easy to test them on CI and VMs.

**6. Not sure how to do something? Just ask.**

If you’re not sure how to do any of this, or anything else, ask before you spend a weekend on it. Our [Discord](https://discord.gg/FUzqGM2cvf)already has a dev channel just for this, and a lot of devs use it every day to ask questions, bounce ideas around, or ask us how to do something. Don’t be shy. Come hang out with us instead of trying random things you’ll end up rewriting later.

None of this is meant as a complaint. I want to make things easier for players and save you from tech debt we all have to deal with later. If you have thoughts or objections, or you want help adding any of this to your project, comment here. I’m happy to help.

Thanks for everything you build, and I truly appreciate you all 🍻To a future where every game has a native decomp/recomp of its own!

**orlandosix** · 393 · 2026-11-17T23:17:11.000Z · `t1_19sslzbdqo7lv`

>**2. Make portable the default.**

dear god please, stop installing to my appdata folders

> **pel14** · 69 · 2026-11-17T23:20:56.000Z · `t1_19ssm5bnvo51h`
>
> unbelievably frustrating and feels like a simple tweak that most (vibe) coders are ignoring

> > **andobrah** · 88 · 2026-11-17T23:24:54.000Z · `t1_19ssmbpruxjes`
> >
> > Not even vibecoders specifically. Everything does this and it's fucking annoying lol. For the last 20 years of using PCs I've wished for this haha

> > > **Capable-Sky-8995** · 37 · 2026-11-18T00:41:00.000Z · `t1_19ssppwq6ug02`
> > >
> > > Every fucking thing should be portable, always.

> > > > **marquesini** · 1 · 2026-11-18T11:10:21.000Z · `t1_19sthsfw2uf7n`
> > > >
> > > > imma drop a shout out to
> > > >
> > > > <https://github.com/fody/costura>

> > > **sid\_killer18** · 27 · 2026-11-18T01:29:36.000Z · `t1_19ssrvx55bibm`
> > >
> > > My fucking appdata folder is over100 GB. What is in there? Fucking EVERYTHING. Python? Check. Discord? Check. WSL?! CHECK. DOCKER?!! CHECK!!! GOOGLE CHROME? YES SIR! MY FUCKING SAVE GAMES? YEAH! NVIDIA SHADERS? YOU GOT IT.

> > > > **Capable-Sky-8995** · 17 · 2026-11-18T01:33:59.000Z · `t1_19sss2ykvg1hd`
> > > >
> > > > I don't even mind every recomp having their own Python. Just stay off my fucking C drive.

> > > > **andobrah** · 5 · 2026-11-18T01:54:20.000Z · `t1_19ssszn1x2hl1`
> > > >
> > > > Glad I'm not the only one that goes insane from this haha backing up and formatting is such a pain in the ass sometimes

> > > > **FullMotionVideo** · 5 · 2026-11-18T02:12:00.000Z · `t1_19ssts0o0b7gj`
> > > >
> > > > All user profile files spread to the wind and people ask what folder to back up. I don't know how MacOS manages to avoid this sort of whining since it's applications are sparsebundles following an OS-defined profile location just like %AppData% and $XDG_CONFIG_HOME.

> > **Nearby\_Ad4786** · 3 · 2026-11-18T06:15:01.000Z · `t1_19st4m87g6r28`
> >
> > Vibe coders? I have lost hundreds of hours with from software games for the same lol

> **Bireus** · 12 · 2026-11-18T00:13:40.000Z · `t1_19ssoi0lzelc2`
>
> The Windows special of save files and configurations

> **Xarishark** · 24 · 2026-11-17T23:18:24.000Z · `t1_19ssm19ce1h1c`
>
> painful isnt it?

> **VALIS666** · 10 · 2026-11-17T23:41:14.000Z · `t1_19ssn1x4gyao1`
>
> I was ranting about this here yesterday as well. Like half(?) of the 50 or so ports I've tried do this. Why? There's just no reason everything can't be in the same folder, it's not Windows XP here.

> **MoonlitEmber** · 3 · 2026-11-18T01:52:19.000Z · `t1_19ssswel6r1j5`
>
> As soon as I start sorting a recomp I start looking at my c drive. Like I'm downloading a zip called [game title] putting it on my 2tb drive. Setting everything to that drive... Boom c drive

> **Poet-Different** · 3 · 2026-11-18T00:15:16.000Z · `t1_19ssokl5kh7y9`
>
> I make all my recomps portable!

> **BigheadSMZ** · 3 · 2026-11-18T00:38:36.000Z · `t1_19sspm1zdmqv5`
>
> This 100%. Nothing irritates me more than when recomps try to install to AppData or Documents.

**Hostile\_18** · 72 · 2026-11-17T23:22:25.000Z · `t1_19ssm7psw46md`

Version Update from within the app/launcher is the biggest one for me.

> **Joecascio2000** · 19 · 2026-11-17T23:40:21.000Z · `t1_19ssn0hxg4c5d`
>
> Yes and this should include updating via Proton (if windows) and Gaming mode for SteamOS.

> **Critical\_\_Hit** · 1 · 2026-11-18T06:34:39.000Z · `t1_19st5hr06aqgy`
>
> Quiver do this.

**Top\_One3087** · 45 · 2026-11-17T23:37:40.000Z · `t1_19ssmw73vokuc`

The second point is absolutely essential: all the data should be in the same folder, clear and organized. Besides, deleting these "recomps" is a real pain; there are so many files that even my NVMe takes an enormous amount of time to delete them. It took me over 30 minutes just to delete the \*DK Bananza\* recomp, I really hope they’ll improve this as well.

And above all, those who create multiple ports for a single game should focus all their efforts on just one or two PC ports at most; people make the most progress when they join forces, that’s as old as the hills.

> **Wyrthoxir** · 32 · 2026-11-17T23:51:42.000Z · `t1_19ssniqvjra0x`
>
> DKB Recomp contributor here. I'm writing up an issue on the DKB Recomp repo that goes through the points; we'll look into tackling them.

> > **Top\_One3087** · 9 · 2026-11-18T00:35:21.000Z · `t1_19sspgtph5dvo`
> >
> > It would be nice.

> **data-atreides** · 4 · 2026-11-18T00:04:25.000Z · `t1_19sso35rutoch`
>
> RomFX and ExeFS are monsters! We need to standardize containing them into a bin file or something

> **unfnknblvbl** · 8 · 2026-11-17T23:48:12.000Z · `t1_19ssnd3s2yq2q`
>
> My god, I hate this so much!! And imo it's the mark of a project using AI without due care. All of my own Claude projects defaulted to using appdata, which on Linux and Android is a nightmare* for the end user to manage. It's really not hard to tell it to expect and/or keep data files in ./data for easier management!
>
> *Ok, maybe not a nightmare, but it's unpleasant

**MagicTurt1e** · 38 · 2026-11-18T05:57:47.000Z · `t1_19st3ujm1kznl`

I'm the Dev of Quiver Launcher and I agree with all of these points too. These are things that will make every launcher experience better for the end user, and gives us launcher devs relief to work on other important features that help improve the space

> **Monika\_Just\_Monika\_** · 1 · 2026-11-18T07:40:00.000Z · `t1_19st8eomwxwg3`
>
> The only thing that I wish your launcher did better because I love quiver is with recomps that get mad at us for having long folder names. I had to essentially build the super Mario Sunshine folder itself and then I had to have quiver search for it because whenever quiver built it itself, it would make it too long. I didn't discover until afterwards that you can have the folder be named something.

> **Critical\_\_Hit** · -4 · 2026-11-18T06:31:51.000Z · `t1_19st5d9hwn7nm`
>
> Guys, can we (gamers) ask you to support ~~private~~ self-hosted repos? [https://www.reddit.com/r/decomps/comments/1xjo6n1/comment/19smdlx5wc1mo/](https://www.reddit.com/r/decomps/comments/1xjo6n1/comment/19smdlx5wc1mo/)

> > **Monika\_Just\_Monika\_** · 3 · 2026-11-18T07:38:50.000Z · `t1_19st8cst5p478`
> >
> > Open source is trustworthy. Not saying close source isn't but it's just a lot harder to trust that it could be

> > > **Critical\_\_Hit** · 4 · 2026-11-18T09:13:29.000Z · `t1_19stcksoga2dc`
> > >
> > > I used "private" incorrectly in my previous message. I meant "self-hosted."

> > > > **Monika\_Just\_Monika\_** · 1 · 2026-11-18T09:50:09.000Z · `t1_19ste7nhewyv5`
> > > >
> > > > Oh we actually that makes sense. Yeah I like the self-hosted ones

> **puntja** · -3 · 2026-11-18T09:14:00.000Z · `t1_19stclmomddab`
>
> Quiver Launcher turned to the worst.. so many things that shouldn't be in there.. like 7 different ports of command and conquer and repos that haven't been updated for years and every single PC Mini game.. it feels so cluttered..
>
> I switched to recomp launcher.. far better experience.. sorry

> > **Monika\_Just\_Monika\_** · 2 · 2026-11-18T09:50:47.000Z · `t1_19ste8oqi5g1t`
> >
> > I don't know. I kind of like the fact that there's multiple on there because then you can look at them test them. You can see which ones worked. Which ones didn't. Which ones are better? Which ones are worse? I like it for testing reasons

> > **maxtinion\_lord** · 2 · 2026-11-18T11:09:24.000Z · `t1_19sthqwoqmnic`
> >
> > I would rather they show everything available than let quiver be the arbiter of what recomp I choose.

**M4rshst0mp** · 30 · 2026-11-17T23:27:16.000Z · `t1_19ssmfhv80ww0`

My biggest grievance right now as a novice users is I need a way to tell when a project requires the prerequisite ROM files to continue to run, and when it builds something entirely new and I can delete the original ROM.

I fear I'm just doubling my disk space with each install

> **Xarishark** · 17 · 2026-11-17T23:28:43.000Z · `t1_19ssmhtswy16t`
>
> And you should not have to. if a build is required the project should fill a folder next to the executable with everything it needs and never have to ask for it again unless you want to change something

> > **kidflashD** · 1 · 2026-11-18T10:07:00.000Z · `t1_19steypofsrr7`
> >
> > Piggybacking off of OP, if the recompilation project dies, and a new one is active, you will likely still need the original ISO.
> >
> > It would be great if there was a commonly accepeted file structure!
> >
> > Edit: I am making my own launcher that will hopefully fix this issue.

> **ThatBlokeYouKnow** · 10 · 2026-11-18T00:05:14.000Z · `t1_19sso4gw2dh8j`
>
> just move the iso out to different folder then start game, if its starts you can delete, if not put it back.

> > **Ok-Efficiency614** · 0 · 2026-11-18T07:52:53.000Z · `t1_19st8zdbzf2pv`
> >
> > its not that, some still require the iso etc for rebuilds, some will just not load if the iso is missing.
> >
> > Take Age of calamity as an example, you need to keep the xci/update/dlc files, it builds the exe and packs the rom into one file then builds the dist folder. so my folder has 3 versions of one game in it.

> **Hostile\_18** · 8 · 2026-11-18T00:04:38.000Z · `t1_19sso3hvolw90`
>
> I also have to keep doubles of every rom. If an update goes wrong, or a project abandoned or another port becomes much better, you need to know what files are the untouched game files (if there are infact any untouched game files left). It's not always easy to tell, especially for extracted games.

**4nimaKlk** · 53 · 2026-11-17T23:21:29.000Z · `t1_19ssm67bkccjp`

Thanks for everything you’re doing for this community. I will be updating my MLB The Show 26 recomp to meet these standards!

> **Xarishark** · 32 · 2026-11-17T23:23:22.000Z · `t1_19ssm98b1usua`
>
> No, thank you. Without Devs like you I would not have a launcher to work one or play the games I love

> **Datmammon** · 2 · 2026-11-17T23:28:04.000Z · `t1_19ssmgsrqdzep`
>
> I watched my first ever MLB matches and postseason this year, and I liked it. Perfect timing for someone to port an mlb game to pc, thank you sir!

**BusBoatBuey** · 18 · 2026-11-17T23:32:14.000Z · `t1_19ssmngx1gmf4`

Also, please don't use the TEMP path. You don't need it. I don't want anything more in there. Leave it alone.

> **Xarishark** · 16 · 2026-11-17T23:36:27.000Z · `t1_19ssmu90yqk5d`
>
> Exactly what I mean with do everything in your executables folder and nowhere else. Believe me. I feel you point here

**Capable-Sky-8995** · 18 · 2026-11-18T00:43:01.000Z · `t1_19sspt54rr1c1`

I would add OpenGL support for **anything** older than Xbox 360/PS3 to the list.

Older devices shouldn't be locked out from running Xbox/PS2/GC recomps just because they don't support Vulkan.

> **woasnoafsloaf** · 3 · 2026-11-18T07:28:07.000Z · `t1_19st7vlkyv1c3`
>
> >Older devices shouldn't be locked out from running Xbox/PS2/GC recomps just because they don't support Vulkan.
>
> Or because they don't have a AVX2 capable CPU. Came across a few of those projects as well.

**full\_of\_wisdom** · 38 · 2026-11-17T23:13:48.000Z · `t1_19ssltvaz3uys`

Pls everyone upvote this and ask under every recomp posted to adopt these standards

**Strong-Wind-6557** · 16 · 2026-11-17T23:29:40.000Z · `t1_19ssmjck72av5`

Precompile option so it doesnt stutter. This is the major one most ports ran fine for me but the stutter issue due to shader compilation is what put me off. Also a reting system for each recomp or decomp is mandatory at this point so people can choose easily.

> **Xarishark** · 12 · 2026-11-17T23:32:40.000Z · `t1_19ssmo5uj88zm`
>
> Most of the time a precompilation step means someone else has to scan those calls and put them in a library so they can be shared on the next project update and its next run(to be clear I dont mean shared shaders just their calls). So the dev themselves would have to play the game until the end to hit as much shader calls as possible, its not something that can be uploaded (easily) them otherwise.

> > **Strong-Wind-6557** · 5 · 2026-11-17T23:34:49.000Z · `t1_19ssmrmbq2kjk`
> >
> > No i meant there was an option for certain ports which helped precompile shaders on our own system before launching the game. Helped a lot tbh when i played them.

> > > **ralphpotato** · 6 · 2026-11-18T03:05:59.000Z · `t1_19ssw6owv7ix1`
> > >
> > > That’s basically what u/Xarishark is saying. Pre-compiling shaders is really dependent on a lot of things. Engines like unreal have systems to precompile shaders, but for an arbitrary game targeting an arbitrary graphics driver, you generally need to play the game to see what shader calls are going to be hit.
> > >
> > > Steam does a version of this where it’s able to collect what those calls should be from the playerbase and then it can distribute that information to other players to let their systems precompile shaders for their setup. On a known system like the Steam Deck they can just ship already compiled shader caches.
> > >
> > > So for a lot of these games it’s not possible to know what shaders need to be precompiled. The data for that has to be gathered from at least some amount of gameplay.

**ocxtitan** · 14 · 2026-11-18T00:40:43.000Z · `t1_19ssppfudawzp`

Create a launcher that is updatable in app is an absolute must, I don't want to have to creep GitHub for hours finding which have updated

> **Xarishark** · 5 · 2026-11-18T00:42:18.000Z · `t1_19ssprzc7wrk2`
>
> Thats fine but its should also be skippable that people that mass manage their projects with apps like RecompLauncher

**DropTheMike13** · 12 · 2026-11-17T23:43:50.000Z · `t1_19ssn63lnhwn7`

Appreciate what you’re doing. Been loving the launcher.

**Miserable\_Initial732** · 10 · 2026-11-18T00:48:21.000Z · `t1_19ssq1ovjnlab`

[u/JasonMaliceMizer](https://www.reddit.com/user/JasonMaliceMizer) [u/chicagogamecollector](https://www.reddit.com/user/chicagogamecollector) Anything the sub could do to help in this regard?

Setup an automoderator to new posts reminding authors of some of these? Or at least pin this post?

What OP's doing is a public service to the decomp community.

> **chicagogamecollector** · 5 · 2026-11-18T00:49:22.000Z · `t1_19ssq3ca2h4e9`
>
> We can look into it. Automod is usually stupid aggressive and everyone will get notified even if they post a project they just stumbled upon

> > **Miserable\_Initial732** · 5 · 2026-11-18T00:53:38.000Z · `t1_19ssqa6gmpu2q`
> >
> > Agreed!
> >
> > Maybe more on something like an automod for new posts. Someone makes a new post with release/recomp/decomp flair, the automod could shoot a reply reminding some of these.
> >
> > Any help is appreciated, though! Thank you

> > > **chicagogamecollector** · 5 · 2026-11-18T00:54:25.000Z · `t1_19ssqbfe3yuma`
> > >
> > > For now I stickied it

> > > > **Miserable\_Initial732** · 5 · 2026-11-18T00:55:03.000Z · `t1_19ssqcgaqyjr7`
> > > >
> > > > Great!

**FilterUrCoffee** · 8 · 2026-11-18T02:02:20.000Z · `t1_19sstchpcja83`

Don't worry, I got you. Recomp of 3d dot game heroes here, Cubic.Heroes is my project. Lots of testing left to do before release, but it's been designed using open source libraries so I can easily make it compatible with windows and Linux. It originally was started on my laptop running Manjaro, but I moved it to my gaming pc just cause it has more power. But once I feel the full game is ready for release I will make sure Linux runs perfectly on my laptop.

> **Xarishark** · 4 · 2026-11-18T02:06:44.000Z · `t1_19sstjkcaekqs`
>
> I really apreciate that! And I would also appreciate you apply the rest of the points too <3

> > **FilterUrCoffee** · 1 · 2026-11-18T02:44:31.000Z · `t1_19ssv87rzykcl`
> >
> > I got you friend! I'll make sure everything works as requested for flexibility sake.

**chaZ04** · 5 · 2026-11-17T23:59:44.000Z · `t1_19ssnvme1ylj4`

Can you highlight which games don't do this in the launcher yet so we know?

> **atlasraven** · 8 · 2026-11-18T00:33:10.000Z · `t1_19sspdbro724i`
>
> Carmageddon 2 is an example of what not to do.

**Inorioru** · 6 · 2026-11-18T00:38:35.000Z · `t1_19sspm08e6wzl`

For 5 I'd like to add:
— In case linux is very low priority for you, at least consider Proton support! Say to your AI something along the lines: Look at code of ProtonTricks and Heroic Game Launcher and add a Proton runtime support.
This way you will be able to focus on Windows version and still bring joy to penguin brotherhood.

> **Xarishark** · 5 · 2026-11-18T00:41:27.000Z · `t1_19sspqm78tg1x`
>
> Fyi on RL we package proton ge and run your non native games in the by default!

**HBum187** · 7 · 2026-11-18T04:52:30.000Z · `t1_19st0xqah4qyq`

Thank you for this.  I think this is a fantastic idea.  I'll be updating all of my projects to adhere to these rules. I've just finished updating UltraCombo and I'm pushing it to GitHub right now!

**battler624** · 7 · 2026-11-18T10:27:59.000Z · `t1_19stfweup7s05`

Thats all great honestly.

I do want one more thing from the devs to focus on but its very hard to do, performance.

Its weird seeing a recomp perform worse than emulator, we need a performance baseline per console compared to most common setup.

For example, all 8th gen console ports should be easily run at their default resolution/framerate on recent 8-core CPU.

Heck, the Xbox one X CPU is equivalent to an FX8300 and its GPU is equivalent to a 1060-6GB or an RX580 both of these are ancient by current standards

A 3600/10400F coupled with a 1070/2060 on Nvidia or RX5600XT on AMD (or better) should get baseline performance factoring in any overhead from the translation layers.

> **RisingPhil** · 1 · 2026-11-18T10:36:12.000Z · `t1_19stg9m8qsg3p`
>
> ,Yes, I agree. I don't care about AI being used. But the result should be to have better performance than an emulator. That's the minimum baseline. Otherwise, what's the point?

**chaZ04** · 14 · 2026-11-17T23:20:45.000Z · `t1_19ssm50y13s76`

Please sticky this mods!!

**Joecascio2000** · 15 · 2026-11-17T23:38:03.000Z · `t1_19ssmwtgmpx8j`

1000% to all of this. Please don't dismiss any of this because you think it is coming from a biased perspective since they develop a launcher. Everything they said, regardless of if it makes their launcher work better, is what I prefer as a Linux and Windows Gamer.

> **Xarishark** · 8 · 2026-11-17T23:45:55.000Z · `t1_19ssn9fuha6x0`
>
> ALL launchers work better not just ours with those points. I would love to have to focus on just making our launcher better instead of having to manage special cases and paths every 5th recomp project ads. with a standard system we would not even have to count on the json files we PR to the devs we work with. we would just count that the save files are always in the save folder and just use that or that --skip  always starts the game directly without ever showing the launcher etc etc. We just need standards bcs right now we are wasting too much time on defragmented logic creation!

> > **michael8684** · 1 · 2026-11-18T06:26:10.000Z · `t1_19st544pyrog2`
> >
> > Love the work you’re doing. Hoping ROMM eventually integrates recomps but single executables are essential for that to happen

**Still-Oil-5355** · 10 · 2026-11-18T01:18:26.000Z · `t1_19ssrdzpxbfyb`

Please add a way to quit games that doesn't involve alt-tabbing, some of us have only a wireless mouse and controller permanently hooked up to our gaming PCs

> **Xarishark** · 6 · 2026-11-18T01:22:53.000Z · `t1_19ssrl4yaw03o`
>
> its like they are built for PC or something lol

> > **Still-Oil-5355** · 1 · 2026-11-18T07:03:14.000Z · `t1_19st6rne3haip`
> >
> > Ironically I have my gaming PC set up like a console. Lol.

**Mr\_NASTY89** · 10 · 2026-11-18T01:36:31.000Z · `t1_19sss71hipqx1`

We need a way to use compressed ROMs. Since all of my ROMs are compressed, decompressing them all would be extremely tedious. My SSD would also appreciate that .

> **Captain\_Chrome\_** · 2 · 2026-11-18T01:48:11.000Z · `t1_19sssprrbqyys`
>
> 💯🙏

**SnooEpiphanies9055** · 4 · 2026-11-17T23:38:50.000Z · `t1_19ssmy2ig1ldu`

now make this a skill . md for the AI bros :)

**MoldyPond** · 3 · 2026-11-17T23:57:15.000Z · `t1_19ssnrninsbya`

Built-in CRT shaders and/or .slang support.

The only three I know of that have them are the Dr. Mario 64, Diddy Kong Racing and Mario All-Stars recomps.

**pl4y3r\_on3** · 3 · 2026-11-18T01:21:25.000Z · `t1_19ssriso0qb2b`

This should be pinned

**DecalMan** · 3 · 2026-11-18T01:44:50.000Z · `t1_19ssske7khfup`

Hey OP. I love Recomp Launcher!! Can you one day add a way to create "custom folders" in the library? So we can create folders and put the recomps in them? Thanks!!!

**Hopalongtom** · 5 · 2026-11-18T00:28:47.000Z · `t1_19ssp6akazamc`

Personally I'd like people to stop using an external compiler like cmake, it's effectively impossible to use on SteamOS, so any that require it is out of the question for me to be able to use

> **Xarishark** · 1 · 2026-11-18T00:33:39.000Z · `t1_19sspe3r7qn43`
>
> you can use brew no?

> > **Hopalongtom** · 2 · 2026-11-18T00:35:40.000Z · `t1_19ssphblfq0zo`
> >
> > Not sure what that one is, I've yet to come across a Recomp asking for that one. If it requires installing required resources through the konsole though probably not.

**Hailogon** · 4 · 2026-11-18T08:32:42.000Z · `t1_19staraol66bk`

Thanks so much for taking the time to post this dude. I'm investigating implementing your first point into my own project based on how Lost Odyssey does it, but the thing I can't work out is that if that project is shipping ready-made release packages then surely it's shipping recompiled proprietary game code? And is therefore in breach of copyright, no?

> **Xarishark** · 1 · 2026-11-18T08:43:49.000Z · `t1_19stb95gd1x1h`
>
> Hey send me your git maybe I can help you

> > **Hailogon** · 1 · 2026-11-18T09:04:03.000Z · `t1_19stc5mp5985f`
> >
> > Oh cheers, thanks for the offer. I’m about a day away from a major release so I’ll ping it over then, thanks!

**throwaroo202020** · 6 · 2026-11-18T00:13:13.000Z · `t1_19ssoh9w1r9mq`

Can we also agree

- fullscreen borderless should be an option and the default.
- reverse vertical look aim MUST be an option.
- PC gamers dont want to see options like button A or require the keyboard to move around menus. We use a thing called a mouse.
- PC recomps should feel like PC games, not console games on a PC.
- saving a game means pressing F5 not saving to a non existent memory card.
- why are you building exes on my PC?

**sm0kah0lic** · 4 · 2026-11-18T00:29:31.000Z · `t1_19ssp7gwq9hfo`

As a fellow Linux developer, I agree on every single point. Sadly for me, I do not own a single Windows computer in my house.

So I have to rely on others to fork my Linux builds.

Appimages are basically the future for portability and every single project I release will have the src in .tar.gz format and Appimage launcher.

After reports of steamdeck users installing games to SD Card, I have begun adding a --install DIR switch to they can install to /run/media/deck/ instead of /home/deck/.local

> **Xarishark** · 5 · 2026-11-18T00:35:33.000Z · `t1_19ssph5akp0g0`
>
> the problem I am facing now with appimages is that I need to extract them to the myriad of projects that dont package a tarbal that is a final project and instead package the source incorrectly tagged...
>
> I am working on getting RL to properly run in flatpak so we can submit to flathub and this whole thing is a mess.

**Combatxlemming** · 2 · 2026-11-17T23:56:43.000Z · `t1_19ssnqsxus876`

All damn good points

I got another one to add-on to this some just say any game disk and some will say nothing at all and then need a super specific one.

Make it easy like game us/EU works and others might not.

**goawaynowpls** · 2 · 2026-11-18T00:27:39.000Z · `t1_19ssp4gr8s4n8`

YES

**Awkward\_Pay\_354** · 2 · 2026-11-18T00:46:04.000Z · `t1_19sspy0sufipu`

This is wonderful! 100% yes to everything. With your ties to Bazzite, can we expect to see RecompLauncher on the Bazaar in the near future?

> **Xarishark** · 2 · 2026-11-18T00:50:01.000Z · `t1_19ssq4dy03vk2`
>
> Bazaar is Flathub. and 2 weeks ago I joined the RL project with this post. [https://github.com/elripalda/RecompLauncher/issues/5](https://github.com/elripalda/RecompLauncher/issues/5)
>
> I will let you draw your own conclusions

> > **Awkward\_Pay\_354** · 1 · 2026-11-18T00:57:37.000Z · `t1_19ssqgl39pxfk`
> >
> > Message received. Thanks for what you do for those of us in the Bazzite community!

**Checho-73** · 2 · 2026-11-18T01:08:37.000Z · `t1_19ssqy930is1s`

Also support all regions of the game, all my files that I already had for emulators are from the USA region so it's annoying seeing when a project only supports EU files.

**emaqqi** · 2 · 2026-11-18T01:32:09.000Z · `t1_19sss00jwcpz6`

As a developer (backend not gaming) really appreciate this, given your position in the community, you could think like a SKILL for agents and give it a go. Like the matt-pocock skils, but specialized for decomps/recomps/ports/emulation. It will be adding more work, but it could go a long way

**fog13k** · 2 · 2026-11-18T01:44:46.000Z · `t1_19ssskanjxrwh`

I'd like people to use PAL/European ROMS to make these recomp as much as possible because they cover way more languages.

Also better to offer the option to detect games both from ISO and from extracted files.

But that default portable mode really needs to happen !

**SgtElectroSketch** · 2 · 2026-11-18T03:08:00.000Z · `t1_19ssw9xzwdc3l`

Dev of VS-Native port chiming in:\
[We should standardize.](https://imgs.xkcd.com/comics/standards.png)

All Jokes aside:

1. I agree, we should not be asking users to build things. Though I provide and extraction script that is triggered from the launcher.
2. Agree, but some things, I want the user to be able to set a path.
3. No Judgement, I've never messed with tarballs.
4. I agree with.
5. I don't know that we can mandate this. Many people are vibe coding, they may not know how to set up a VM to test their project in a Linux environment. And then, what flavors should they test in? Bazzite, sure, steamOS, Fedora, Ubuntu, Mint, etc...
6. Can the community please stop asking immediately "Are you going to port to Android/MacOS/iOS, I've put in a couple hundred hours of my own time working on my project, and when the very first question is porting, it makes me want to throw my damn keyboard out the window. Granted I'm not doing a recomp, I'm doing a full native source port of the game. \*A LOT\* of time has gone into it.
7. We should be providing a roadmap, what the project is, what we plan to do once it's working, and how we plan to support it.
8. For the love of god, build in an updater.

**GoHamInHogHeaven** · 3 · 2026-11-18T00:00:12.000Z · `t1_19ssnwdtyswox`

Just want to say, avoiding multiple.exes isn't a real option for me. My MediEvil port is heavily optimized, and one of those optimizations is a seperate dev build that compiles out unnecessary tooling, this has a measurable performance improvement. Things like AVX2 or AVX512 also require having seperate executables, and I don't really think it's possible do dynamic dispatching in this instance. The users runs the application with a script and not an .exe anyhow, so maybe that's fine for you guys. Offering separate download links for different builds it possible obviously, but may be even more confusing for the player :/

The rest I can do. Outside of a couple of settings, my in-game menu (which opens right from the games pause menu) has everything.

I don't think my fork of the original repo is on this launcher yet, but it probably should be, because I update it regularly with new features and bug fixes.

> **iku\_19** · 7 · 2026-11-18T00:08:47.000Z · `t1_19ssoa66og7lt`
>
> That's not what OP meant with multiple executables. They are talking about bootstrapping executables.

> > **Xarishark** · 5 · 2026-11-18T00:11:56.000Z · `t1_19ssof7ztn6o5`
> >
> > Exactly. The entrypoint should arrive with the users download. Not created on buildtime.

> > > **GoHamInHogHeaven** · 3 · 2026-11-18T00:36:59.000Z · `t1_19sspjfqpshvk`
> > >
> > > Got it, I see what you're getting at now. That's not a problem.

> **Xarishark** · 1 · 2026-11-18T00:07:22.000Z · `t1_19sso7wicq0sh`
>
> Your entrypoint can be one exe tho what you spawn after that does not hold you hostage to the exe build. you could just call the two paths for each build and play step without having either in the exe code(in a sense). The point here is to make one entrypoint for everyone. so things are clear. literally every app does that. If you drop me your git I could take a look also. Feel free to submit your project until then: [https://recomplauncher.com/submit?platform=pc](https://recomplauncher.com/submit?platform=pc)

> > **ocassionallyaduck** · 2 · 2026-11-18T01:12:15.000Z · `t1_19ssr435ilz0k`
> >
> > I would rephrase point 1 then, because I don't use any of the 37 various recomp launchers, but games themselves often also ship with a "launcher", so I had no idea what you were on about. Having a stubbed .exe that launches the main process is fine, but you make it sound like they need to upload something with more pseudo game code or something. It's unclear is all.

**FullMotionVideo** · 4 · 2026-11-18T02:07:48.000Z · `t1_19sstla4shabn`

Most aren't "scattering files around your home folder", they're following XDG Base Directory spec.

I used one AppImage that ignored $XDG_CONFIG_HOME dumped a ton of support files in whatever directory it was executed out of. This made managing it with Gear Lever obnoxious because Gear Lever puts every .appimage in a common folder (akin to MacOS /Applications) and that folder is getting flooded with non-appimage files as a result of a misbehaving program that wanted to "replicate our behavior on Windows."

> **Xarishark** · 1 · 2026-11-18T02:12:47.000Z · `t1_19sstta8mddaa`
>
> By a floating binary not installed by a package manager. A recomp appimage or tarbal is not a standard app and should be fully self enclosed in its folder. Now an emulator flatpak or whatever is a totally other story ofc.  If a recomp comes from a package manager then I am more than happy it uses whatever static dir they have submitted. But running the appimage of hookshot should give you a single dir to set and never write anywhere else other than that.

> > **Ok-Opportunity-1979** · 1 · 2026-11-18T07:12:36.000Z · `t1_19st76o9w7o91`
> >
> > I'm with you in this case. When I compile a recomp last time I was having a hard time thinking where to put things like shaders and control maping fixes in a portable way to change platform to a rpi4, for example. I think this is the way. One game one folder otherwise will be a hell transfer them.

> > **FullMotionVideo** · 1 · 2026-11-18T02:32:40.000Z · `t1_19ssup77wz72c`
> >
> > I understand what you're saying. I'm telling you that you're wrong. As MacOS apps are bundled that write in the user's Preferences folder, as Windows apps have AppData directory, $XDG_DATA_HOME defaults to ~/.local/share on most distros and programs are supposed to put their config files there, regardless of whether they're installed by a package manager or compiled from source. Programs are supposed to always be ready to be put somewhere among other programs in a PATH location (such as /bin, /usr/share/bin, or ~/.local/bin).
> >
> > AppImage specifications does have a [portable mode](https://docs.appimage.org/user-guide/portable-mode.html) that uses folders in the same working directory with names like GameRecomp.appimage.config and store their files in there. It is fair to ask developers to take advantage of this feature, which is 95% of what you want since the files are under the appimage's working directory. But what you're complaining about is supposed to be the default behavior.

> > > **Xarishark** · 3 · 2026-11-18T03:03:31.000Z · `t1_19ssw2q19jkap`
> > >
> > > XDG\_CONFIG\_HOME defaults to \~/.config you are talking about XDG\_DATA\_HOME
> > >
> > > Recomps are a bit of a special case because what they write isnt just config. Its GBs of assets extracted from your rom and sometimes a whole built game. When a binary someone ran from Downloads leaves gigabytes in \~/.local/share and theres no uninstaller to ever clean it, thats what I mean by scattering. So follow XDG by default sure , respect the variables properly and give us one flag or env var to put everything under a single dir. That way a launcher or a portable setup can keep it all in one place and everyone is happy.

> > > > **FullMotionVideo** · 1 · 2026-11-18T03:19:05.000Z · `t1_19sswrqg856yt`
> > > >
> > > > Yeah, I had my variables reversed. I have a friend on Discord complaining about the same thing as you do, I just think that portable mode solves this issue and devs should do more to encourage it's use. That way you put your huge games in `<name>.AppImage.home/.local/share` or `<name>.AppImage.config` and it works.
> > > >
> > > > The downside to portable mode is that the folder needs the exact same filename as the appimage, and the appimage names change if they include version numbers. That's where you as the dev of a launcher/manager could assist by tracking updates and renaming them (Gear Lever also does this) so that the portable folders work consistently.

> > > > > **Xarishark** · 1 · 2026-11-18T03:32:59.000Z · `t1_19ssxe1s02lqc`
> > > > >
> > > > > On the universe of recomps the true downside is that it’s Linux specific. A -data flag is cross platform instantly. Again I’m only taking about recomps as I said what you are taking about is totally viable for literally everything else. I developed the old and the new  bazzite portal and I exclusively use xdg always.
> > > > >
> > > > > Also keep in mind that I am taking from the side of managing thousands of projects with a single launcher and more than half don’t follow any convention at all.
> > > > >
> > > > > Right now I am trying to prepare RL for flathub myself. And 90% of projects are appimage only with no tarball. So what I have to do is extract the appimage because fuse is forbidden in flatpak.
> > > > >
> > > > > Running the project you like as a standalone appimage is fine if it bundles a pretty zsync for your gear lever. But I have to extract that appimage or pr a ci change myself and pray they haven’t abandoned the git.
> > > > >
> > > > > Hell

**SuperBio** · 2 · 2026-11-17T23:59:00.000Z · `t1_19ssnugvk7lvl`

Have you considered creating a matrix server in addition to your discord? I know a lot of people interested in recomps that aren't going to use Discord due to the security issues, arbitrary nuking of servers, and other issues.

**paulgdp** · 2 · 2026-11-18T00:21:18.000Z · `t1_19ssou91ew39g`

I'm surprised XDG base directories and XDG user directories ain't mentioned. They are the standard locations for config, data, state, cache, documents etc on Linux.

Native linux games and all apps in general use those directories.

> **Xarishark** · 2 · 2026-11-18T00:28:40.000Z · `t1_19ssp63nxtqww`
>
> Because of request 2. This is not getting installed by a package manager. And XDG is used by most appimages hence the directory pollution

> > **paulgdp** · 1 · 2026-11-18T00:33:59.000Z · `t1_19sspem9r58qr`
> >
> > Oh I thought you meant portable for the game only.
> >
> > I guess that's fine too but I personally prefer those files going to their usual place.
> >
> > An option to choose portable vs xdg would satisfy everyone then!

**Miserable\_Initial732** · 2 · 2026-11-18T00:43:53.000Z · `t1_19sspuiqqnhj4`

Off topic, but thank you so much u/Xarishark for the initiative.

Y'all are as important to the PC-porting scene and all it represents in terms of digital freedom as the decomp devs themselves. What you guys are doing with RecompLauncher, organizing everyone and tidying everything is leadership stuff from the history books.

RecompLauncher will soon be to PC Ports what Retroarch is to emulation, and your names will be on wikipedia soon enough.

Thank you so much for what you've been doing! Keep it up!

> **Xarishark** · 3 · 2026-11-18T00:47:01.000Z · `t1_19sspzk0qmh6q`
>
> I appreciate your kind words! Only think I hope personally is for us all to have proper standards that are shared and known by everyone, in the case someone doesnt support them people would then ask them to use them!

**newhacker1746** · 2 · 2026-11-17T23:31:45.000Z · `t1_19ssmmp5ol284`

I've been in some RE scenes and was around for the original sm64 decomp's release, and its really tragic how so much dev and hacking scene that was originally a very linux and community oriented thing has given way to the first screenshot chasing windows only monolithic builds.

I really do believe that's why theyre windows only; it caters not to the best development cycle or ecosystem but to the platform that gets the most attention

**NumberSevere7454** · 1 · 2026-11-18T00:36:30.000Z · `t1_19sspiojellky`

where there are not xbox one games in this launcher?

> **Xarishark** · 1 · 2026-11-18T00:37:34.000Z · `t1_19sspkdyoynb4`
>
> feel free to send me the git of any and we will be happy to add them!

**Andrew\_hl2** · 1 · 2026-11-18T00:54:50.000Z · `t1_19ssqc4cgy7i8`

![gif](giphy|eunrMjB8lBUKeL1fqD)

**Nosereddit** · 1 · 2026-11-18T01:42:20.000Z · `t1_19sssgdhq0f41`

yes please it only make it easier for every1

**alefjan** · 1 · 2026-11-18T02:40:13.000Z · `t1_19ssv1b6mhw1t`

Having a white paper that is shared between all recomps/ decomps with best practices and documentation is the way to go.
The person who did recompia has made an amazing job( probably because he is a very good programmer) and I believe everything he achieved should be replicated in all decomps and recompensa with documentation. Get together yall you guys have an amazing path toward the future of gaming restoration and conservation.

**Wolf-Wolfy** · 1 · 2026-11-18T03:00:34.000Z · `t1_19ssvxzxmgrr8`

Portable is a must,

Bro i love your launcher and what are you doing for our scene,

I know it's near impossible but i would love your launcher works as anker launcher and just with one click able to play

**Wolfen459** · 1 · 2026-11-18T03:06:04.000Z · `t1_19ssw6u7l7fcw`

Portable Mode 100% !
Last time I asked for this there were a lot of questionable comments from people. It almost felt like I insulted their egos. 🤷

**Wolfen459** · 1 · 2026-11-18T03:10:40.000Z · `t1_19sswe7g6k7sx`

I would put another thing to the first point.
Having a dedicated exe to skip the Launcher and go straight to the Game.
I don't understand how, but there are almost only a handful of Ports that actually let you do this.
They all have multiple exe's in the folder, yet every single one just still opens the Launcher.

**DifferentProcess6765** · 1 · 2026-11-18T03:32:54.000Z · `t1_19ssxdwiz97r7`

100% agree

**originaljunglistuk** · 1 · 2026-11-18T03:40:53.000Z · `t1_19ssxqqfpb3b6`

Could we get an option on recomplauncher that just says portable for all the ones that are?

**vxz-z** · 1 · 2026-11-18T03:52:48.000Z · `t1_19ssy9vocfsld`

thank you for trying to set a standard, i think the recomp scene is a bit of a wild west right now, i personally have a launcher for all my mcp and tools because its easier for me to work on, the building the game is just a scrip\[t but the launcher is just the gui, if everyone just adds that script to recomp launcher as a pack you we could remove the need for each game launcher as a whole and you could just build it inside recomp launcher, add firmware once, my software autodetects and builds any psvita or ps3 game (obviously not optimised for unique stuff) if that system is inside of recomp it would cut down on waiting to download then recompile, you could just scan your whole library and then it could show what recomps are out for your games inside just one launcher, having it just be a script as a pack would make it less of a legal issue for me

**Anchovie123** · 1 · 2026-11-18T03:57:29.000Z · `t1_19ssyhegd6qv9`

Also please focus on performance and optimization before adding niche mod features that barely anyone will use. I think most people just want to play the game in high resolution and with high framerates. Niche features like multiplayer mods and randomizers ect can come later.

**WillyMyPete** · 1 · 2026-11-18T04:48:28.000Z · `t1_19st0r8u90bd0`

Thank you everyone for making these recomps of games from my childhood I never got to play ❤️❤️

**CiggiesInside** · 1 · 2026-11-18T05:27:11.000Z · `t1_19st2hetohmys`

you'd do well to master a set of skills for llms that enforce these asks

**aiat\_gamer** · 1 · 2026-11-18T06:18:02.000Z · `t1_19st4r26l3erp`

Now that we are here, the longshot app that runs automatically inside the launcher installs the game in my C drive, I did not see an option to manually choose a different drive.

**nerso02** · 1 · 2026-11-18T06:18:38.000Z · `t1_19st4s0pwjypt`

If I can add something. If your game has a launcher please make it controller friendly for people on Steam Deck/Machine or htpc. Even something simple like press A to start game, and other settings only mouse and keyboard.

**macumbamacaca** · 1 · 2026-11-18T07:17:12.000Z · `t1_19st7e2a9ot4y`

Someone make a skill for this!

**Majestic\_Doctor\_2** · 1 · 2026-11-18T09:35:31.000Z · `t1_19stdk6djdmkg`

Thank you for all of this, especially 1 and 2 lol

**maddietendo** · 1 · 2026-11-18T10:28:55.000Z · `t1_19stfxwwnv3ia`

I’m the dev on Decompbuddy who’s archiving every decomp ever for safe keeping so I see a lot of them… and this is a good list. It would go a long way to help the community. Love this.

**cob59** · 1 · 2026-11-18T10:31:07.000Z · `t1_19stg1gdtkf44`

I have limited knowledge about decomps, but I know software.

What you need right now is to figure out and publish a manifest specification for a small versioned file (JSON or whatever) that contributors can put in their repo to describe everything you've listed in that letter and more. Any heavier standardization approach would fail I think, but a single manifest should be added either by the project owner or a PR and list info such as:

- Project ID, upstream repo and license
- Release asset naming per OS/architecture, plus checksums
- Required original game files, identified by hash and the asset extraction step
- Paths for saves, config and mods
- A minimal CLI contract for a single exec: --version, a headless install/extract mode, and meaningful exit codes
- Depending on the license, maybe an opt-out flag if they don't want to be listed in your launcher or another...

Then a conformance validator for that manifest. An open CLI and a GitHub Action that contributors drop into CI to check that the manifest is valid, that release assets exist and match their checksums, that the binary launches, and that no copyrighted assets ship in the release.

**ishtaracademy** · 1 · 2026-11-18T10:34:41.000Z · `t1_19stg75su7pfk`

Excellent stuff. I personally feel like there should also be a ground rules for recomps that an AI like Claude could easily reference and implement given other sample repos. I think by this point, we should have...

- In game options menus. Hate having to quit game to change resolution or stuff.
- QOL enhancements. Skip cutscenes or fast forward or change things like encounter rate, rare drops, etc.
- save anywhere. We don't need to be restricted to the old save points. But we can still use it if we want.
- 16:9 and 21:9 where realistic (like some 2d games just won't and I get it)
- shader support
- texture replacement support

Recomps like Weltall (xenogears) are amazing. See that one for a good example. Or the burnout 3 recomp that brings all the options into the game itself.

I don't like downloading a recomp just to see it's using that psx or SNES wrapper launcher. Lame. It's no better than an emulator with those lazy recomps.

**wdcossey** · 1 · 2026-11-18T10:43:08.000Z · `t1_19stgkqa1vvk3`

Nice post, however it would probably be easier if the decomp authors had tools to use, rather than some specs \[in a Reddit post\].

Some options:

1. Make a small SDK that the decomp authors could use, to better integrate with said launcher.
2. A prompt/rules file their local agents could ingest to guide them. Something to describe requirements and conventions, so the agent does the integration for them.
3. A hosted MCP server that their agent can connect to, it could serve ground rules and templates, guiding the agent(s) to build out the required features.

Option 2 is probably the easiest to start with.

> **Xarishark** · 1 · 2026-11-18T12:21:00.000Z · `t1_19stkxw59a64k`
>
> RL literally has all that

**Werewolf\_Capable** · 1 · 2026-11-18T11:13:18.000Z · `t1_19sthx60ovsld`

Sane words

**slumslanger64** · 1 · 2026-11-18T01:05:22.000Z · `t1_19ssqt0e5wakl`

OP, I would consider reaching out to all the '""leaders""" of the recomp community, hashing out and finalizing these rules, and coordinating a spread of it on social media

> **Xarishark** · 3 · 2026-11-18T01:18:33.000Z · `t1_19ssre6f6ie85`
>
> I would be more than happy for this to happen. this would indeed be the proper way for things to get in order.

> > **slumslanger64** · 2 · 2026-11-18T01:38:16.000Z · `t1_19sss9u7abvk0`
> >
> > goodluck !

**StrategicRebootEngi** · 1 · 2026-11-18T01:56:33.000Z · `t1_19sst37dhsaro`

Cheers to all of this. Particularly the Linux consideration <3

> **Xarishark** · 2 · 2026-11-18T02:05:00.000Z · `t1_19sstgrgwio3o`
>
> Soon we will all be free!

**Background-Hyena2387** · 1 · 2026-11-18T04:03:19.000Z · `t1_19ssyqqye9bwj`

Your program is amazing but your concept of "we" in the Internet wild west is naive.

**P-Pablo** · 1 · 2026-11-18T04:23:15.000Z · `t1_19sszmrdnbret`

Well, my word will not have the same weight as yours, but let me say something

I'm a software developer with years of experience mostly on web development, but also a gamer also with years of gaming. I follow many open source projects and I'm active in some communities. I've started to clone and save many git repositories since the lawsuit of Rockstar Games to the devs of re3/revc (the revernse enginner project of gta 3 and vice city) for preservation purposes.

So, when the boom of the decomp/recomp started I was excited (and im still excited) about many projects available, it makes me to rejoice many games with new options and accesibility. But one thing i cannot stand and still i cannot understand is WHY YOU MAKE A PROJECT CLOSED SOURCE". Want to take all the credit? Wanna have all the control of the development or distribution? Want to avoid people like me who thinks by cloning your code is stealing? I simply I dont get it. I'm talking for example the nfsu2 web and xrevive (the midnight club 3 recomp for pc) which by making their project closed source I feel unsecure and with my hands tied in case of preservation (the companies will sue soon or later) or improvements.

Right now im unemployee, I'm studying, with no money to pay an IA to at least assist me in a decomp or recomp project and I have little time to dedicate it on learn how to decomp/recomp/reverse enginner something, but at least I give kudos and try to give something on those projects who still are open source and are open to work with more people, for example OpenUG2, mc3dub website, Vice City Stories PC, Takedawn (the pc recomp of Burnout 3) and many, many other projects.

**ValuableBig8717** · 0 · 2026-11-18T00:00:09.000Z · `t1_19ssnwahm70xu`

Longshot se tornou nas últimas 24 horas o melhor recomp do Zelda Ocarina.

Mas ele rende quase a querer virar a estrela do projeto e não o JOGO EM SI.

Agressivo demais, a APPDATA virou bagunça agora?

Até o executável do jogo no Launcher leva para AppData no botão jogar.

Sendo que na pasta há um botão de executável direto.

Os menus estão ótimos\
O jogo tá fluido\
Tá tudo quase beirando a perfeição

MAS DEIXE O JOGO SER A ESTRELA de tudo SEMPRE!

Eu uso caminhos para o meu Launcher, Hydra, que renomeia executável\
Cria ícones próprios idênticos do console\
Artes e afins são baixadas etc\
Fica perfeito\
Mas nem todos sabem fazê-lo mesmo sendo simples

Eu não quero jogar Longshot ou N2XNavi eu quero jogar ZELDA.

E eu sou grato ao Longshot Project por isso ser possível.\
E eu falarei deles.\
Mas o jogo ainda é o principal aqui!

Após o jogo estar lá, o Launcher tem que sumir.

Atualizações?\
Ótimo, podem ser incorporadas ao iniciar o game um aviso para acessar o Launcher a atualizar quando elas estiverem disponíveis é só

> **Xarishark** · 7 · 2026-11-18T00:08:56.000Z · `t1_19ssoaesvu9he`
>
> you should talk to the devs and ask them to fix this then. hookshot and longshot both spread on multiple folders + temp during their build. That I know of from multiple reports.

> > **ValuableBig8717** · -1 · 2026-11-18T00:26:32.000Z · `t1_19ssp2nuh0pkx`
> >
> > Eu varri os arquivos, coloquei pra caça-los, monitoro diretórios, e eu uso meu Windows 24/7 em modo de test e desenvolvimento, então sou criterioso, pra ter uma noção aqui estão desligados qualquer mitigação de KVA Shadown Melltdown ou Spectre só de início, obviamente isolamento de núcleo e integridade de memória, digo para entender como priorizo meu Windows limpo, fiz um programa para monitor muitas coisas e estado de todas as pastas ocultas ou não e por baixo da pele.
> >
> > Então quando me deparei com uma verdadeira sujeira absurda que o NX2Navi fez imaginei que o Longshot poderia seguir o padrão
> >
> > Ele conseguiu ser pior kkk\
> > O jogo?\
> > Rodou maravilhosamente bem\
> > Sou muito grato\
> > Está perfeito
> >
> > Mas não encontrei necessidade da bagunça que fez na máquina sem necessidade kkk
> >
> > Limpei tudo, mas tem que ficar atento.
> >
> > Não menos importante, QUANDO ATUALIZAR o Launcher precisa saber o que abrir mão.
> >
> > Eu tinha um recomp com 26gb semana passada num recomp\
> > Atualizei e tinha 35gb\
> > Estranhei a real necessidade, desinstalei e reinstalei do zero a mesma versão atualizada\
> > E fiquei com 26,5Gb
> >
> > Alguém me explica? Haha

**ValuableBig8717** · 0 · 2026-11-18T00:09:04.000Z · `t1_19ssoamezg5fl`

Como eu tenho feito?

Vou jogar o novo Zelda?\
Crio uma pasta exclusiva para ele escondida no meu disco secundário

Deixo a rom NPU em OUTRO disco (isso evita saturar IOs e divide trabalho nos discos) busco a rom no disco 1 e compilo, cria-se a pasta do jogo no disco 2.

Deleto toda sujeira (deveria haver um botão para isto).

Aponto o executável para meu Launcher pessoal padrão (uso o Hydra Launcher e já usei Playnit e Steam) mas deveria haver botões nos recomps para enviar o exe para eles, integrando aos Launchers que usamos de fato. Não quero 50 diferentes para cada recompo e jogo.

Depois no Launcher do Hydra eu peço para baixar artes, capas, metadados, tempo de jogo etc\
E mando criar executável na área de trabalho corretamente e não genéricos e na steam.

Pelo meu Launcher início muito mais rapidamente os jogos Recomps como se eu estivesse em seus consoles respectivos que usando da forma que os devs aqui parecem querer que eu faça rs

Depois, eu preciso SEMPRE passar a tesoura na pasta do jogo que foi criada.\
9 em cada 10 jogos eu posso usar o CompactGUI com compactação padrão do Windows\
Só no Zelda Ocarina Of Time eu economizei claramente 7GB de disco sem perdas, sem nada perdido na fidelidade visual ou desempenho, nem mesmo tela de carregamento. Exatamente mesmo jogo, desempenho e visual com 20% menos espaço em disco ocupado.

E se pretendo não ligar pro espaço e quero desempenho máximo, rodo o SCS Killer para compilar Shaders ao máximo possível até o fim do jogo, com opção experimental para evitar travamentos ao mudar de cenários

Poderia ser uma opção integrada nos projetos.\
Funcionou bem com Zelda e Mário aliás.

Mas depois tive que fazer malabarismo para encontrar o lixo que o SCSKiller traduziu direto na pasta da MS (3DSCache) e deixar apenas a que Nvidia já leu construiu e usará de fato.

**khsh01** · 0 · 2026-11-18T01:52:13.000Z · `t1_19sssw8xt0o05`

Unfortunately you're talking into a sea of mostly vibe coders and clout chasers and I don't know which is worse but neither are devs. And what you're asking for is minimal qol stuff that a real dev would just include by default like Lost Odyssey

> **MaleficentNobody100** · -1 · 2026-11-18T02:11:26.000Z · `t1_19sstr3hvcolf`
>
> Why are you here if you are so hostile lmao

> > **khsh01** · 0 · 2026-11-18T02:13:32.000Z · `t1_19sstugpv7i4k`
> >
> > So I should just spew copium like the rest of you?

> > > **MaleficentNobody100** · -1 · 2026-11-18T04:58:25.000Z · `t1_19st178kai9he`
> > >
> > > Epic dichotomy bro.

> > **psychedelic\_tech** · 0 · 2026-11-18T10:16:04.000Z · `t1_19stfdaepf5hc`
> >
> > they aren't wrong at all.

**Eastern-Bluejay-8912** · 0 · 2026-11-18T03:28:17.000Z · `t1_19ssx6ietom0z`

I completely agree. 100% this. In fact, mods, can we please pin this post? lol 😂 An while you do not have a complaint, I do. Before starting a project, search on Reddit and Google if it is already a live project. If so, ask to join their team. If not, announce your project intentions and keep updates coming while you work on it. We don’t need 4 different decomps of Mario kart world. 🤣 Or ocarina of time. Especially when some decomps and recomps are starting to pop up with legit hacking viruses in them. Stay grouped and keep people updated so people know you are legit and will praise your effort.

**EbolaBoi** · 0 · 2026-11-18T07:43:03.000Z · `t1_19st8jl5sdbb5`

Amen!

**Defaalt** · 0 · 2026-11-18T12:06:31.000Z · `t1_19stkamypw9s4`

Lol! That's great but no!\
It's bigger than any of us. Peopme with absolutely zero coding skills are making stuff! You can't force them to follow a handbook !\
Every project will be different. Use it as it was shipped or make your own.

**zeroz41** · -1 · 2026-11-18T01:28:35.000Z · `t1_19ssruaincyv7`

i agree in some ways,

if shipping portable, all should run inside the folder, statically linked, including saves.

If i specifically choose to also ship a flatpak or exe installer in addition, this is purposeful that i want saves in standard hidden folders such as \~/.local or appdata, and i specifically want the application to be installed system wide, including a desktop file that appears on your start menu.

so basically im not disagreeing at all, but the comments i read here that say "EVERYTHING SHOULD ALWAYS BE PORTABLE EVERYTIME" really confuse me on how ignorant it is.
