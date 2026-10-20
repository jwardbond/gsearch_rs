# Methodology: LaTeX text

Copy the `latex` block. It continues from "defining the distance function in
\cref{e:spatial-alignment} to be a weighted sum of the distance between all paths and".
The text uses the labels `alg:match-generation` and `alg:path-scoring`.

Notes:

- Your text refers to `e:spatial-coincidence`. The label of the equation is `e:spatial-alignment`.
  Fix one of them.
- The end-point terms are in `dist`. They are not in `path-scoring-latex.md` yet. That file has
  one node residual $s_V$ for all nodes. The two are close, but they are not the same. Make
  them match before you submit.
- The bound on $T$ (Procrustes is near the best $T$) is not checked in detail. The text below
  does not claim it.

```latex
    We approach this problem by defining the distance function in \cref{e:spatial-alignment} to be a weighted sum of the distance between all paths and
    the area between the edge and the path, and the distance between the end points of the edge and the matched vertices. For an edge $(u,v) \in E_Q$ and a path $P$ in $G$,
    \begin{align}
        \label{e:dist}
        \text{dist}(T(u,v), P) = \lambda\, A\big(T(u,v), P\big) + \|T(u) - \phi(u)\| + \|T(v) - \phi(v)\| ,
    \end{align}
    where $A$ is the area between the path $P$ and the straight segment from $T(u)$ to $T(v)$, and $\lambda$ sets the weight of the area against the end-point distance. A path that wanders far from the segment has a large area. A path that does not reach the matched vertices has a large end-point distance. The end-point terms do not depend on $P$. For this reason, the best path for each edge is the path with the smallest area, and we find it with Dijkstra's algorithm.\footnote{We approximate $A$ by the trapezoid rule over the edges of $P$, and we ignore nodes of $G$ that are farther than a tolerance $\varepsilon$ from the segment. See \cref{alg:path-scoring}.}

    Finding $\phi^*$ and $T^*$ together is hard, so we split the search into three steps.
    \begin{enumerate}
        \item \textbf{Candidate matches.} We build a small set of injective maps $\phi$ with a spatial index on $G$ (\cref{alg:match-generation}). We keep only the maps that can reach a low score.
        \item \textbf{Rigid transform.} For each $\phi$, we find $T$ with the Procrustes method. This has a closed form by SVD and allows reflections. It minimizes the squared distance between the matched vertices only, so it is a good estimate of $T^*$, but it is not exact.
        \item \textbf{Edge paths.} For each edge of $Q$, we find the best path in $G$ and add the costs (\cref{alg:path-scoring}). If one edge of $Q$ has no path inside the tolerance, we reject $\phi$.
    \end{enumerate}
    The match with the lowest total $S$ is the result. Only the last step is exact. The first step can remove a good $\phi$ if the tolerance is too small, and the second step can return a $T$ that is not optimal.
```
