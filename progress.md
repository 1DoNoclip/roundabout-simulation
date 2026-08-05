Documenting progress here until Diary is available again.

Entry: 04/08/2026
* Write the initial pathfinding algorithm.

Todo:
* Fix order that start and end points are determined. The pathfinding is failing because the start and end points are not always possible to travel between.
- [x] Temporarily attempt pathfinding always using lane_index = 0.
- [x] Rewrite pathfinding to use context of arms instead of BFS.
- [x] Work out the desync issue for sectors (and why routes go from entry deflection to intra sector, not inter sector).

Entry: 05/08/2026
* Rewrite pathfinding to use knowledge of arm indexes instead of breadth-first search. Reduces the chance of hidden bugs and panics due to invalid states.
* Fix invalid inter sector segments due to incorrect indexing, meaning pathfinding and segments should now be correct.

Todo:
* Implement zone system.
* Use zone system to determine lane choice.