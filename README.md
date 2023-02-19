# gdtree
CLI tool for displaying a Godot scene in a tree-like format

While learning how to use Godot, I wanted a way to compare example projects for reference without having to keep closing and opening them just to look at the scene tree.  Viewing the scene file directly isn't very helpful for large scenes, especially ones with big meshes with lots of vertices.  This is a simple tool that prints it out in hierarchical form, similar to the Unix-like tree command.  Originally, I wrote it in Python but it struggled with large scenes.  I rewrote it in Rust and now it's about 50x faster.

## Installation
Download a pre-built binary or run `cargo build`.

## Example
```bash
./gdtree godot-demo-projects/mono/dodge_the_creeps/Main.tscn
Main
│   * script: res://Main.cs
├── ColorRect
│       * anchor_right: 1.0
│       * anchor_bottom: 1.0
│       * color: Color( 0.219608, 0.372549, 0.380392, 1 )
├── Player
├── MobTimer
│       * wait_time: 0.5
├── ScoreTimer
├── StartTimer
│       * wait_time: 2.0
│       * one_shot: true
├── StartPosition
│       * position: Vector2( 240, 450 )
├── MobPath
│   │   * curve: Curve2D
│   └── MobSpawnLocation
├── HUD
├── Music
│       * stream: res://art/House In a Forest Loop.ogg
└── DeathSound
        * stream: res://art/gameover.wav
```

