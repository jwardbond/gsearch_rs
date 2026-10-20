# Path scoring: LaTeX text

Copy everything inside the `latex` block into your document. It matches the style of your
match-generation text (same macros: `algorithm`, `algpseudocode`, `cleveref`).

Source: `../gsearch/src/gsearch/align_edges.py` (`align_and_score`, `_align_graph`, `_find_paths`).

Notes on what the Python does, so the text stays true to the code:

- The node residual is the Frobenius norm (not squared).
- Reflections are allowed in the Procrustes step (`det R` can be `-1`).
- A node is a candidate for an edge only if it is within `tol` of the aligned segment. If any
  query edge has no path, the whole match is rejected (`None`).
- The code computes `score = node_score + weight * edge_score`. The docstring says
  `weight * node_error + (1 - weight) * edge_cost`. The code is what the text below uses.
  Fix one of them.

```latex
\begin{algorithm}[h]
\caption{Path Scoring}
\label{alg:path-scoring}
\begin{algorithmic}[1]
\Require Query $Q$, target crop $G$, matching $\phi : V_Q \to V_G$, tolerance $\varepsilon$, edge weight $\lambda$
\Ensure Score $s$, transform $T$, subgraph $H \subseteq G$, or $\bot$ if $\phi$ cannot be aligned
\State $T \gets \arg\min_{T \in \mathrm{E}(2)} \sum_{v \in V_Q} \|T(x(v)) - x(\phi(v))\|^2$ \label{line:procrustes}
\State $\hat{x}(v) \gets T(x(v))$ for all $v \in V_Q$
\State $s_V \gets \big( \sum_{v \in V_Q} \|\hat{x}(v) - x(\phi(v))\|^2 \big)^{1/2}$ \label{line:nodescore}
\State $s_E \gets 0$, $\; H \gets$ the graph with nodes $\phi(V_Q)$ and no edges
\For{$(u, v) \in E_Q$}
  \State $\ell \gets$ segment from $\hat{x}(u)$ to $\hat{x}(v)$
  \If{$\mathcal{P}_\varepsilon(u,v) = \emptyset$} \Return $\bot$ \EndIf
  \State $P \gets \arg\min_{P \in \mathcal{P}_\varepsilon(u,v)} c(P)$ \label{line:shortestpath}
  \State $s_E \gets s_E + c(P)$
  \State add the nodes and edges of $P$ to $H$
\EndFor
\State \Return $\big( s_V + \lambda\, s_E,\; T,\; H \big)$
\end{algorithmic}
\end{algorithm}

\Cref{alg:path-scoring} is a rigid alignment followed by a corridor shortest path. The matching
$\phi$ from \cref{alg:match-generation} fixes where the nodes of $Q$ land in $G$, but not how
the edges of $Q$ appear in $G$. We first fit the rigid transform $T$ (line \ref{line:procrustes}),
which has a closed form by SVD (Sabata et al., 1991). Reflections are allowed, because
\cref{alg:match-generation} also accepts reflected matches. The residual $s_V$ measures how far
the matched nodes are from a perfect copy of $Q$.

For each edge $(u,v) \in E_Q$, $\mathcal{P}_\varepsilon(u,v)$ is the set of paths in $G$ from
$\phi(u)$ to $\phi(v)$ whose nodes all lie within $\varepsilon$ of the moved segment $\ell$.
The cost of a path $P = (w_0, \ldots, w_m)$ is the area between $P$ and $\ell$, by the
trapezoid rule:
\begin{align}
    c(P) = \sum_{i=1}^{m} \tfrac{1}{2}\big( d(w_{i-1}, \ell) + d(w_i, \ell) \big)\, \|x(w_i) - x(w_{i-1})\| ,
\end{align}
where $d(w,\ell)$ is the distance from $w$ to $\ell$. The best path is found with Dijkstra's
algorithm, after removing the nodes of $G$ that are farther than $\varepsilon$ from $\ell$. If
any edge of $Q$ has no path, $\phi$ is rejected. A lower score $s = s_V + \lambda\, s_E$ is a
better match.
```
