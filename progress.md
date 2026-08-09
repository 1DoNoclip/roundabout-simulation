Documenting progress here until Diary is available again.

Entry: 04/08/2026
* Write the initial pathfinding algorithm.

Todo:
* Fix order that start and end points are determined. The pathfinding is failing because the start and end points are not always possible to travel between.
* [x] Temporarily attempt pathfinding always using lane_index = 0.
* [x] Rewrite pathfinding to use context of arms instead of BFS.
* [x] Work out the desync issue for sectors (and why routes go from entry deflection to intra sector, not inter sector).

Entry: 05/08/2026
* Rewrite pathfinding to use knowledge of arm indexes instead of breadth-first search. Reduces the chance of hidden bugs and panics due to invalid states.
* Fix invalid inter sector segments due to incorrect indexing, meaning pathfinding and segments should now be correct.

Todo:
* ~~[ ] Upload missing progress videos to OneDrive (2026/08/05, ...).~~
* [x] Complete encapsulation and conversion of to pub(crate).
* [x] Move tests/ into lib.rs's tests as these are not testing a public API.
* ~~[ ] Implement zone system by copying existing implementation in assembly::calculate_destination_weights.~~
* ~~[ ] Use zone system to determine lane choice.~~

Entry: 06/08/2026
* Work on improving the code to make it easier to add features in the future. This included editing structures so that they are easier to understand and use and creating a VehicleBundle instead of spawning all the components of a vehicle manually.

Todo:
* ~~[ ] Upload missing progress videos to OneDrive (2026/08/05, ...).~~
* ~~[ ] Implement zone system by copying existing implementation in assembly::calculate_destination_weights.~~
  status: implemented a different system instead
* ~~[ ] Use zone system to determine lane choice.~~

Entry: 08/08/2026
* Add lane selection system, allowing vehicles to use different lanes based on their exit. Note: I did not implement the zone system, instead I compare the arm index difference, not the arm angle difference. This ensures maximum lane utilisation (e.g., if the 2 other exits are below 180 degrees, this system ensures that the right lane is used for the 2nd exit).
* Add CLI (command line interface) arguments support. One flag (--paused / -p) allows me to run with the simulation paused initially.

Todo:
* [ ] Replace existing OneDrive videos with Kdenlive exported versions.
* [ ] Upload missing progress videos to OneDrive (2026/08/05, ...).