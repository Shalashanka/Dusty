# Dusty: manual test checklist

Everything requested so far, as things to try. Tick them off as you go.
Tip: **Settings › About › tap the version 7 times** unlocks Developer mode,
which can fire notifications, digests and scans on demand.

## New in 0.3.0

**Updates**
- [ ] Settings › About › Updates shows the current version and the source (default `C:\dev\Dusty\releases`)
- [ ] Publish a newer build with `scripts/release.ps1 -Version 0.3.1 -Notes "test"` → "Check now" finds it; Settings shows a dot badge
- [ ] "Update & restart" installs without uninstalling and reopens Dusty on the new version
- [ ] With auto-install on and the window closed, Dusty updates itself after ~10 min idle; otherwise a toast offers "Update now"
- [ ] Tamper test: edit one byte of the installer in `releases/` → update is refused (bad signature)

**Voice** (Settings › Voice)
- [ ] Turn voice on, download Moonshine (~100 MB, progress bar); optionally Parakeet (~640 MB, Italian and 24 more languages)
- [ ] `Ctrl+Alt+D` anywhere → bar opens with the Siri-like ribbons that react to your voice → stop talking → transcript + chips → results
- [ ] Result arrives within about a second of finishing speaking
- [ ] Talk back off by default; when on, Dusty reads a one-line answer with a local Windows voice
- [ ] Without a model downloaded, the voice bar explains how to get one

**Search**
- [ ] Combined terms don't fail: "invoice pdf last year", "photos of the trip to rome", "setup exe in downloads"
- [ ] Typos and accents: "invocie", "perche", "fotografie" still find things
- [ ] Italian: "trova le foto delle vacanze dell'anno scorso", "video grandi sul disco d"
- [ ] App folders (AppData, node_modules, .git, build) rank below your own files

**Command palette & bar**
- [ ] `Ctrl+Alt+K` with Dusty only in the tray → command list (scan, pause, clean the sure ones, empty bin, views)
- [ ] Typing `>` in the search bar switches to commands
- [ ] All three hotkeys can be re-recorded in Settings; conflicts are reported
- [ ] The bar has a soft blurred (acrylic) background and resizes to its content
- [ ] Onboarding step 4 "Dusty at your fingertips" mentions the palette and voice keys

**Tray menu**
- [ ] Right-click the tray icon → Dusty-styled menu with icons and status; Escape or clicking outside closes it
- [ ] Tray icon sweeps (animated) while scanning

**Alive Dusty**
- [ ] Home: dust motes drift; Dusty blinks, follows the pointer, fidgets now and then; thought bubbles appear occasionally
- [ ] Click Dusty → hop + hearts
- [ ] Tidy: removing a card leaves a dust puff
- [ ] Settings › Dusty's presence › "Wave 👋" → Dusty walks along the bottom of the screen, says something, walks off; clicks pass through
- [ ] With "wander" on, a stroll happens at most every few hours, never in fullscreen or quiet hours

**Fixes**
- [ ] Space no longer shows boxes for files deleted since the last index (updates within seconds)
- [ ] Tidy badge in the sidebar matches the number of items (no more 99+ with 7 left)

## Fixes in 0.2.1
- [ ] Buttons near the top (Space drive chips, refresh, Tidy view toggles) are clickable; the fade under the title bar no longer blocks them
- [ ] After installing, the window opens on its own (not only in the tray), even on a slow first start
- [ ] Never an empty window: if something fails at startup, a "Try again" message appears instead
- [ ] Onboarding shows once after upgrading (new features), then never again
- [ ] While scanning, Home shows the phase plus elapsed time (e.g. "Hunting for duplicates… 1m 05s")
- [ ] Scans don't start while the index is still being built, and duplicate hashing is capped per scan (no long disk thrashing)
- [ ] App stays responsive during and after scans (menus, switching views)
- [ ] Space on C:\ is smooth to hover and switch folders
- [ ] Duplicate/similar photo on D: and G: → ⋯ › "These folders are backups of each other…" → suggested pair (e.g. D:\Photos ↔ G:\Photos), choose another level or pick folders by hand → findings disappear
- [ ] Settings › Folders & policies › Mirrored folders lists the pair; Remove works; "Add a mirrored pair" with two folder pickers
- [ ] Next scan no longer flags photos inside mirrored folders

## Install & identity
- [ ] Uninstall the old **Dustling** (Settings › Apps), then install `Dusty_0.2.0_x64-setup.exe`
- [ ] App, tray icon, window title and toasts all say **Dusty**; icon shows the mascot with headscarf + broom
- [ ] Your old Dustling history/decisions carried over (History tab)
- [ ] The old "Dustling" autostart entry is gone (Task Manager › Startup apps)

## Mascot & polish
- [ ] Mascot no longer jumps frantically on any screen (onboarding, Home, Tidy, empty states)
- [ ] Mascot has a broom, apron and headscarf; eyes follow the mouse; blinks; sweeps while scanning; sleeps when paused
- [ ] Title bar (minimize / maximize / close) has a gradient; scrolling content doesn't show through harshly
- [ ] Closing the window hides Dusty to the tray; tray click reopens it

## Onboarding (Developer › "Show onboarding again", then restart)
- [ ] 5 interactive steps: hello → Downloads/Desktop rule → extra folders → search hotkey → how hands-on
- [ ] Choosing "Keep it empty" for Downloads creates a folder policy (Settings › Folders & policies)
- [ ] Pressing Ctrl+Alt+Space on step 4 turns the box green
- [ ] "Bold" style puts junk/build junk/archives/empty folders on Autopilot

## Home
- [ ] Drive selector (All / C: / D: / G:) filters the headline, findings and "Free on…" stat
- [ ] Clicking a drive card focuses it; double-click opens it in Space
- [ ] **Free space updates right after deleting** (no rescan needed). Note: *recycled* files still use space until the bin is emptied
- [ ] Recycle Bin card shows its size; "Empty bin" asks twice, then frees space
- [ ] Index card shows file count / indexing progress; "Search" opens the popup

## Tidy (cards)
- [ ] Card shows preview for images, video (plays), audio, PDF, text; Windows thumbnail for others (PSD, folders…)
- [ ] Duplicates and similar photos show **side-by-side** previews
- [ ] **Open** and **Show in folder** buttons on the card (`Enter` / `O` work too)
- [ ] **⋯ › Don't touch this** for a day / week / month / forever, and "the whole folder" options; item disappears, Undo in the snackbar works
- [ ] "Delete for good (skip the bin)" in ⋯ for recyclable items
- [ ] Swipe/drag left removes, right keeps; `←` `→` `↓` `Z` keys work
- [ ] "What's this?" cards (unknown big old folders) read as a question
- [ ] Sleepy project → **Hibernate**: zip appears in Cold Storage, folder replaced by `DUSTY-HIBERNATED.txt`; History › Hibernating projects › **Wake up** restores it
- [ ] Folder plan card → "See the plan" opens the plan editor

## Tidy (list)
- [ ] Group by **Type** (e.g. all `.dll`s together), Reason, Folder, Drive, None
- [ ] Group checkbox selects the whole group; groups collapse
- [ ] Bulk bar **always** offers Remove / Recycle / Delete for good / Keep / Later (no disappearing delete button)
- [ ] "Remove" handles a mixed selection (caches deleted for good, files recycled, …)
- [ ] Right-click a row: Open, Show in folder, actions, Don't touch
- [ ] Recycle vs Delete for good: hover shows the difference (bin + undo vs permanent + immediate space)

## Organizing plans (folder policies)
- [ ] Settings › Folders & policies: add a folder, set "Keep empty" or "Keep sorted", grace period, ask-every
- [ ] A `.bat`/`.ps1` lands in Scripts; installers already installed → Recycle; zips already extracted → Recycle; screenshots → Pictures\Screenshots
- [ ] Change an item's destination/action, or a whole category ("All: Move to…/Recycle/Leave")
- [ ] "Do it" moves everything; snackbar Undo puts it all back
- [ ] Next plan remembers the destination you picked for that file type
- [ ] Items you chose to "Leave" aren't proposed again

## Watched folders & whole system
- [ ] Settings › "Extra folders to watch": old files there get flagged like Downloads
- [ ] Old stuff on other drives (D:, G:) shows up (duplicates, big dormant, "What's this?")
- [ ] Settings › "Drives I look after" toggles drives in/out of the index

## Search (Ctrl+Alt+Space, anywhere in Windows)
- [ ] Popup appears centered, on top; Esc or clicking elsewhere hides it
- [ ] Results while typing in milliseconds; preview pane on the right
- [ ] Natural language: "pdf invoice about 5 months ago", "big videos on d:", "screenshots last week", "budget folder in documents", `"exact name"`, "bigger than 1 gb"
- [ ] Chips show how the query was understood
- [ ] Enter opens, Ctrl+Enter shows in folder, Ctrl+C copies the path
- [ ] Settings › Behavior › Search shortcut: record a different combo, it works immediately
- [ ] A file created after indexing is still found (live watcher)

## Space
- [ ] Click a drive: nested boxes (folders inside folders) appear instantly ("Instant (indexed)")
- [ ] Hovering the list on the right **no longer makes things jump**
- [ ] Right-click a box: Zoom in, Open in Explorer, Copy path, Recycle, Delete for good, Don't flag
- [ ] Known caches are labelled (green = safe, amber = careful, red = leave alone) with an explanation
- [ ] Works on every drive, not just C:

## Speed
- [ ] Memory ring, startup count, suspicious process count
- [ ] Startup apps toggle on/off (machine-wide ones ask for admin); matches Task Manager's Startup tab
- [ ] Live process list updates every few seconds; leftover dev processes flagged ("End leftovers")
- [ ] Optimize a drive (UAC) / "Check fragmentation" on the HDD shows a %
- [ ] Hibernation off/on, Delivery Optimization clear, Component cleanup (UAC), power plan switch
- [ ] "Find preinstalled apps" lists bloat; Remove works

## Notifications (use Developer mode to trigger)
- [ ] "Test notification" shows a Dusty toast with the icon
- [ ] Nudge toast has **"Clean the sure ones (N, X GB)"**: pressing it cleans without opening the app, then confirms
- [ ] "Let's look" opens Tidy; "Not now" does nothing
- [ ] Weekly digest toast ("This week you freed …")
- [ ] Autopilot summary toast
- [ ] No toasts during quiet hours or fullscreen games

## Smarts
- [ ] Keep several files of the same kind (e.g. PDFs from one site) → later ones say "You usually keep…" with lower confidence
- [ ] A new app adding itself to startup produces a "New startup apps" card
- [ ] Clearing a cache location doesn't re-flag it immediately on the next scan
- [ ] Knowledge base explains junk (e.g. "Adobe Premiere Pro cache · usually safe to delete")

## Settings & About
- [ ] Tabs: What I look for · Folders & policies · Behavior · Protected · About
- [ ] Protected tab lists everything you protected; Remove brings it back next scan
- [ ] About shows credits and **Buy me a coffee** (disabled until `coffeeUrl` is set in `src/lib/config.ts`)
- [ ] Tapping the version 7× unlocks the **Developer** tab; it can be turned off again

## Developer tab
- [ ] Test notification · Nudge · Autopilot toast · Weekly digest · Background scan · Rebuild index · Show onboarding · Forget history · Open data folder
