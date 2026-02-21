**MAGICKA® is the property of Paradox Interactive AB.** This project is not associated with or endorsed by Paradox Interactive or Arrowhead Game Studios. See [LICENSE.md](./LICENSE.md).

---

A sandbox playground for exploring and interpreting parts of Magicka, giving players an easier way to experiment with gameplay and content creation, and better modern platform hardware support.

You must have Magicka 1 installed. This project doesn't provide any content, instead it directly loads from your existing copy of the game.  
Set the `MAGICKA_CONTENT_DIR` environment variable to the path of the *Content* directory in your Magicka install.  
On Linux, this is most commonly `~/.local/share/Steam/steamapps/common/Magicka/Content`. On Microsoft Windows, this is most commonly `C:\Program Files (x86)\Steam\steamapps\common\Magicka\Content`.

Inputs and shortcuts:
- W, A, S, D - Move around
- Q, W, E, R, A, S, D, F - Conjure elements
- Mouse - Aim
- Right Click - Forward/force cast
- Shift + Right Click - Area cast
- Middle Click - Self cast
- Shift + Left Click - Cast from weapon as if already imbued
- P - Free camera. In free camera mode:
  - Mouse - Move camera orientation
  - Scroll - Adjust movement speed
  - Left Click - Hold to grab cursor
  - M - Toggle cursor grab
  - W, A, S, D - Fly forwards, left, backwards, & right
  - E, Q - Fly up & down
  - Shift - Fly faster while held
- Backtick (\`) - Console. Notable commands:
  - `help`
  - `scene [--level Havindr] havindr_s2`
  - `trigger "Spawn Orcs Singledoor"`
  - `spawn-character King`
- F3 - Browse & edit entities in the world
- F6 - AI navigation/pathfinding debug overlay
- F7 - Physics debug overlay

## Interests/goals
- Sandbox mode
- Online servers with more than 4 players, including campaign and arena
- Spell system experimentation and rebalancing. What would it be like to play through Magicka with on-release Wizard Wars balance?
- Easier modding?

![Screenshot outside Castle Aldrheim](docs/assets/screenshot.jpg)

## Development
[Bevy](https://bevy.org) is used for ECS, rendering, etc.
