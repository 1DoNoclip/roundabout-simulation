Documenting progress here until Diary is available again.

Entry: 04/08/2026
* Write the initial pathfinding algorithm.  

Todo:
* Fix order that start and end points are determined. The pathfinding is failing because the start and end points are not always possible to travel between.
- [x] Temporarily attempt pathfinding always using lane_index = 0.
- [ ] Rewrite pathfinding to use context of arms instead of BFS.
- [ ] Once working, implement zone system.
- [ ] Once working and validated, use zone system to determine lane choice.