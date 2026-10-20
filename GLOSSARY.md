# gsearch

Search for a small geometric shape (the query) inside a large geometric graph (the target). The result is the places in the target whose local geometry matches the query within a tolerance.

## Language

**Query**:
The small graph `Q` that is searched for. Its coordinates are in a frame of their own.
_Avoid_: pattern, needle

**Target**:
The large graph `G` that is searched in.
_Avoid_: haystack, database graph

**Anchor**:
A node used as the centre of a comparison. The query has one (its first node). Each candidate anchor in the target is paired with it.
_Avoid_: root, seed

**Crop**:
The part of the target around a candidate anchor: the connected component of the anchor among the nodes within a radius.
_Avoid_: window, patch, neighbourhood

**Tolerance**:
The maximum difference in distance, in target units, that still counts as a match. It is also the half-width of an annulus and of a corridor.
_Avoid_: epsilon, threshold

**Annulus**:
The band of width twice the tolerance around the circle that a query node draws when the query is turned about its anchor.
_Avoid_: ring

**Mapping**:
An injective assignment of every query node to its own crop node, such that the distances to the anchors agree within the tolerance. It does not yet say how well the shapes fit.
_Avoid_: pose, match, matching

**Transform**:
The rotation or reflection, plus a translation, that moves the query onto the nodes of one mapping.
_Avoid_: alignment (that is the scored result)

**Corridor**:
For one query edge, the nodes of the crop that lie within the tolerance of the line through the moved edge. Paths for that edge may use only these nodes.
_Avoid_: tube, band

**Alignment**:
A mapping together with its transform, its score and the subgraph of the crop that realises it. The search returns the best alignments.
_Avoid_: result, hit

**Score**:
A number for an alignment, where lower is better. It is the node score plus the weight times the edge score.
_Avoid_: cost, error (for the total)
