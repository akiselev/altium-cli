

# TritonRoute: The Open Source Detailed Router

Andrew B. Kahng, *Fellow, IEEE*,  
 Lutong Wang, *Student Member, IEEE*, and Bangqi Xu, *Student Member, IEEE*

**Abstract**—Detailed routing is a dead-or-alive critical element in design automation tooling for advanced node enablement. However, very few works address detailed routing in the recent open literature, particularly in the context of modern industrial designs and a complete, end-to-end flow. The ISPD-2018 Initial Detailed Routing Contest addressed this gap for modern industrial designs, using a realistic design rules set. In this work, we present TritonRoute, a detailed router capable of delivering a DRC-clean routing solution. The key contributions of TritonRoute include an in-memory router database, along with an end-to-end detailed routing scheme that is capable of comprehending connectivity and design rule constraints, with every key detail revealed by a code release under a permissive open source license. We evaluate our router using the official ISPD-2018 benchmark suite and show that TritonRoute achieves an unprecedented solution quality – improved wirelength and via count, and an extremely low level of design rule violations (DRCs). Compared to the known best detailed routing solutions from all published academic detailed routers, TritonRoute improves wirelength by up to 0.8% (avg. 0.4%), via count by up to 16.1% (avg. 9.3%), and DRCs by up to 100% (avg. 92.0%).

## I. INTRODUCTION

Detailed routing is a dead-or-alive critical element of advanced node enablement. New technology nodes come with smaller feature sizes, while fundamental physical (lithographic patterning, CMP, reliability, variability, etc.) and circuit (crosstalk, delay, etc.) limitations remain. As a result, ever-more complex design rules must be comprehended and satisfied at the detailed routing stage, greatly challenging routability as well as the architecture and strategy of the detailed router itself.

Due to the high complexity and enormous solution space for the VLSI routing problem, the routing is typically split into global routing and detailed routing stages. In global routing, the routing region is divided into rectangular grid cells and represented using a coarse-grained 3D routing graph. Capacities and various constraints are assigned to the edges and vertices in this 3D routing graph so that overall routing topology and layer assignment can be optimized considering routability, timing, crosstalk, power, etc. The ensuing detailed routing stage attempts to realize the segments and vias according to the global routing solution, while minimizing design rule violations.

The detailed routing problem has been extensively studied for more than five decades. The fundamental algorithms (e.g., Lee’s algorithm, unidirectional and bidirectional A\* search,

ripup-and-reroute paradigm, etc.) and problem formulations (e.g., channel routing and switchbox routing) have largely remained intact in commercial tools for several decades; see [3] for a thorough review. These algorithms and formulations are elaborated to meet real-world requirements (design-rule correctness, quality of result, scalability, and turnaround time) and widely deployed in today’s commercial tools that support foundry N7, N5 or even N3 nodes.

However, only a few academic works [11] even attempt to present an end-to-end detailed routing flow, and almost no works make claims to viability in the real-world IC physical design (P&R) context. Since most detailed routing research focus on different objectives, such as crosstalk or new-technology contexts, comparison between these works is difficult. Further, direct application of academic codes to modern industrial benchmarks has many hurdles, especially given that commercial tools and industrial designs satisfy far more, and more complex, design rules than any academic tools.

Given the above, it is a highly significant milestone for the field that the ISPD-2018 contest, on the subject of initial detailed routing, has recently exposed industrial detailed routing challenges and benchmarks to the academic community [25][38]. The ISPD-2018 benchmark suite provides 10 testcases in 45nm and 32nm nodes, with up to 290K standard cells and 182K nets. These designs are industrial benchmarks – including large memory cells, off-track pin access, IO ports, and power and macro blockages – with realistic design rules offered in industry-standard input/output formats while keeping problem complexity tractable to academic researchers within the four-month contest timespan. However, even two full years after the initial release of the ISPD-2018 contest, there are only a few works [4][5][13][18][22][31] capable of delivering any kind of result; these results have nearly a thousand, if not thousands, of design rule check violations (DRCs) for nearly every testcase. Up until now, no work has come close to approaching the solution quality we expect from commercial detailed routers, although almost every work utilizes a variant of the five-decades-old path search algorithm.

Based on the ISPD-2018 Initial Detailed Routing contest, the present paper describes TritonRoute, an open source detailed router for advanced VLSI technologies. Our main contribution is an end-to-end (i.e., complete, and with collaterals visible in a permissively open-sourced repository) detailed routing framework that aims and achieves beyond all existing academic detailed routers. Highlights of our work are summarized as follows.

- We propose an end-to-end detailed routing scheme. Our proposed scheme is capable of comprehending connectivity constraints (i.e., opens and shorts) and design

A. B. Kahng is with the Departments of Computer Science and Engineering, and of Electrical and Computer Engineering, University of California at San Diego, La Jolla, CA, 92093 USA (email: abk@ucsd.edu).

L. Wang and B. Xu are with the Department of Electrical and Computer Engineering, University of California at San Diego, La Jolla, CA, 92093 USA (e-mail: {luw002, bangqixu}@ucsd.edu).

rule constraints (i.e., spacing tables, end-of-line (EOL) spacing, minimum area and cut spacing).

- We build an in-memory router database that complies with LEF/DEF data models. This non-contest-driven code infrastructure enables future development and leverage of our open-source code towards deeper core optimization, more complete design rule support, and other enhancements.
- We present a number of key ideas in addition to the well-known A\*-based path search. Transparency of our descriptions is aided by all implementation source codes being released under a permissive open source license.
- We evaluate our router using the official ISPD-2018 benchmark suite, and show that we reach an unprecedented, extremely low level of DRCs (<20) in seven of 10 testcases, which is a 99.3% reduction of DRCs on average as compared to the known best detailed routing solutions from all published academic detailed routers. For the remaining three testcases, we reduce DRCs by 75.1% on average, and by 60.0% at a minimum. Overall, compared to the known best detailed routing solutions, TritonRoute improves wirelength by up to 0.8% (avg. 0.4%), via count by up to 16.1% (avg. 9.3%), and DRCs by up to 100% (avg. 92.0%).
- To the best of our knowledge, we are the first and the only open source gridded detailed router which is capable of delivering a DRC-clean detailed routing solution in sub-65nm technology nodes.

The remainder of this paper is organized as follows. Section II provides a brief overview of previous works in the open literature. As noted above, such literature is sparse as far as it gives insight into industry routing tools and how they address modern routing challenges. Section III presents our router database. Section IV details our overall detailed routing flow. Section V presents our detailed routing methodology. Section VI presents our experimental results using the official ISPD-2018 benchmark suite. Section VII gives conclusions and directions for ongoing work.

## II. PREVIOUS WORKS

As surveyed in [3], previous works on detailed routing can be categorized into fundamental and conventional algorithms, and recent developments. Further, we summarize the recent works targeting the ISPD-2018 initial detailed routing contest. **Fundamental and conventional algorithms.** Lee [20] proposed the first maze routing algorithm, i.e., a breadth-first search that guarantees to find a minimum-cost path between two terminals if a path exists. Use of “best-first search”, also known as A\* search [27], sometimes in its bidirectional [28] form, enables maze-based search to focus itself toward desired targets, and reduces effort needed to find a minimum-cost feasible path. Hadlock [14] and Soukup [30] applied speedups to Lee’s algorithm and others applied the line-search paradigm [17] to improve time and space efficiency as compared to Lee’s and A\* algorithms. Hetzel [16] developed a sequential routing approach using a shortest path algorithm with respect to euclidean distance. Specialized contexts such

as channel routing [9] and switchbox routing [24], along with general frameworks such as multicommodity flow [29] and ripup-and-reroute [32], have respective sub-literatures and remain as fundamental building blocks of the detailed router today (cf. [11]).

**Recent developments.** More recent academic works on detailed routing focus on certain aspects of the modern routing challenge, mainly to address issues arising with advanced nodes. [21] gives an excellent summary of the academia-industry gap for detailed routing as of 2003; much of this gap remains today. Examples of focused recent works include Nieberg [26], which proposes techniques for gridless pin access in detailed routing. Xu [34] proposes pin-access planning and regular routing for self-aligned double patterning (SADP). The works of [6][8][10][23] address the detailed routing problem in an SADP process context. MANA [2] introduces an end-end separation and minimum wire length-aware shortest path algorithm. Han [15] develops a framework to reduce various DRCs in advanced nodes using multicommodity flow-based integer-linear programming. BonnRoute [1][11] and RegularRoute [35] are two works prominent in the recent literature that present more complete portraits of overall detailed routing solutions.

**ISPD contest-based works.** Recently, a few works in the open literature attempt to address the gap between modern industrial designs and academic detailed routing flows, based on the ISPD-2018 initial detailed routing contest [25]. Sun [31] presents a multi-stage ripup-and-reroute flow for detailed routing. Kahng [18] proposes an integer linear programming (ILP)-based parallel intra-layer and sequential inter-layer routing flow. Chen [4][5] and Li [22] propose a detailed routing flow using min-area-captured path search on a sparse grid graph. Gonçalves et al. [12][13] propose a tunnel-aware A\* lower bound, and a design-rule-aware path search algorithm for detailed routing. Although most recent works use correct-by-construction or safe-by-construction approaches to prevent DRCs, none of them is capable of delivering decent solution quality (that is, in a practical sense) due to the complexity of developing the necessary router infrastructure.

## III. DATABASE

In this section, we list all major objects and structures in the routing database. In building this database, we follow the LEF/DEF [40] data model, and reuse the naming convention from OpenAccess [41] as much as possible. The objects from LEF are summarized in Table I, and the objects from DEF are summarized in Table II. The structure of the database is described in Figure 1. The database is an in-memory, flattened physical design database. In the top level, the database consists of a **technology library**, a **top block** and several **reference blocks**.

### A. Technology library

Technology library stores all metal and cut **layers**, **viadefs**, and design rule **constraints**. A back-of-end-stack layer consists of basic layer information, i.e., type, direction, pitch, offset, as well as all its applied design rule constraints. A

![Figure 1: Major database structures. This diagram illustrates the hierarchical structure of a design database. It is divided into several sections: 'Design' (Top Block, Instance 0-3, Instance Term 0-3, Instance Blockage 0-3), 'Net' (Net 0-3, Seg 0-3, Via 0-3, Patch 0-3), 'Ref Block' (Ref Block 0-3), 'Term' (Term 0-3), 'Pin' (Pin 0-3), 'Shape' (Shape 0-3), 'Blockage' (Blockage 0-3), and 'Tech' (Rule 0-3, Layer 0-3, Via Def 0-3). A central column lists 'Boundaries', 'Track Patterns', 'Gcell Patterns', and 'Markers'.](1b7d539e02a202c2cf2d97698b911447_img.jpg)

Figure 1: Major database structures. This diagram illustrates the hierarchical structure of a design database. It is divided into several sections: 'Design' (Top Block, Instance 0-3, Instance Term 0-3, Instance Blockage 0-3), 'Net' (Net 0-3, Seg 0-3, Via 0-3, Patch 0-3), 'Ref Block' (Ref Block 0-3), 'Term' (Term 0-3), 'Pin' (Pin 0-3), 'Shape' (Shape 0-3), 'Blockage' (Blockage 0-3), and 'Tech' (Rule 0-3, Layer 0-3, Via Def 0-3). A central column lists 'Boundaries', 'Track Patterns', 'Gcell Patterns', and 'Markers'.

Fig. 1: Major database structures.

TABLE I: Database objects from LEF.

| Object     | LEF Keyword  | Meaning                         |
|------------|--------------|---------------------------------|
| tech       |              | back-end-of-line metal stacks   |
| layer      | LAYER        | metal or cut layers             |
| viadef     | VIA          | via definitions                 |
| constraint | WIDTH        | default routing width           |
|            | AREA         | minimum area rule               |
|            | SPACING      | spacing rule                    |
|            | SPACINGTABLE | spacing table rule              |
|            | MINIMUMCUT   | minimum cut rule                |
|            | MINWIDTH     | minimum width rule              |
|            | MINSTEP      | minimum step rule               |
| block      | MACRO        | standard or macro cells         |
| term       | PIN          | standard or macro cell pin      |
| blockage   | OBS          | standard or macro cell blockage |
| pin        | PORT         | physical pin                    |
| rect       | RECT         | rectangle                       |
| polygon    | POLYGON      | polygon                         |

TABLE II: Database objects from DEF.

| Object       | DEF Keyword | Meaning                            |
|--------------|-------------|------------------------------------|
| block        | DESIGN      | block-level design                 |
| inst         | COMPONENTS  | instance of standard or macro cell |
| term         | PINS        | block-level IO pin                 |
| blockage     | BLOCKAGES   | block-level blockage               |
| net          | SPECIALNETS | special net                        |
|              | NETS        | regular net                        |
| instTerm     |             | points to a term                   |
| instBlockage |             | points to a blockage               |
| pathSeg      |             | routing segment                    |
| via          |             | routing via                        |
| patchMetal   |             | routing patch rectangle            |

viadef holds one or more **shapes** (rectangles or polygons) on two consecutive metal layers with shape(s) in the middle cut layer, realizing physical connection between neighboring metal layers at the same  $x$ - $y$  coordinate. We summarize the design rules that we support in Table III. For definitions, examples, and detailed handling methodology of each rule, please refer to [36].

TABLE III: Design rules.

```
// metal layer
WIDTH defaultWidth ;
[MINWIDTH minWidth ;]
SPACINGTABLE
  PARALLELRUNLENGTH {length} ...
  {WIDTH width {spacing} ...} ... ;
[SPACING minSpacing SAMENET [PGONLY] ;]
[MINSTEP minStepLength [MAXEDGES maxEdges] ;]
[SPACING eolSpacing ENDOFLINE eolWidth WITHIN eolWithin
  [PARALLELEDGE parSpace WITHIN parWithin [TWOEDGES] ;] ...
// cut layer
{SPACING cutSpacing [CENTERTOCENTER]
  [ ADJACENTCUTS numCuts WITHIN cutWithin [EXCEPTSAMEPGNET]
  | PARALLELOVERLAP
  | AREA cutArea] ;}...
[SPACING cutSpacingSN [CENTERTOCENTER] SAMENET ;]
```

### B. Block

The top block describes the flattened logical and physical connections, following the DEF model. There are four major types of objects: **term**, **blockage**, **instance** and **net**. A reference block is a standard or macro cell from LEF, having the same data structure as the top block, except that only terms and blockages are populated.

1) **Term**: Terms are IO pins for the top block, and standard or macro cell pins for the reference blocks. Each term consists of one or more physical **pins**. Each pin consists of one or more physical shapes across one or more metal and cut layers.<sup>1</sup>

2) **Blockage**: Blockages are user-defined routing blockages from DEF BLOCKAGES for the top block, and are from LEF OBS statements for reference blocks. We reuse the pin object to hold physical shapes of the blockages.

3) **Instance**: Instances are from DEF COMPONENTS. Each instance is an instantiation of either a standard cell or

<sup>1</sup>A term including more than one pin with “MUSTJOIN” keyword indicates that the two pins should be physically connected in detailed routing. In this work, we assume that each term holds one physical pin, so as to simplify the description.

a macro block, holding zero or more **instance terms** and **instance blockages**. An instance term points to the related term from its reference block. An instance blockage points to the related blockages from its reference block.

4) *Net*: Nets are from DEF NETS and SPECIALNETS. A net stores its logical connections, and its physical connections, i.e., **pathSegs**, **vias** and **patchMetals**. A pathSeg is a point to point routing wire on a specific layer, defined with the start and end points, width and extensions. A via is an instantiation of viadef at a specific coordinate. A patchMetal is a patching rectangular metal used to satisfy various design rules.

Other types of objects in a block include **boundary**, **track-Pattern**, **gcellPattern**, **marker**, etc. The gcellPattern object defines the global routing cells (GCells) [7] in 2D grids;<sup>2</sup> and marker object represents a design rule check (DRC) violation, including the bounding box, layer, violation type and source objects. In our implementation, we also build several assisting objects and structures. Some of the procedures are described in Section IV. A complete picture and details of the database implementation are visible at [37].

## IV. FLOW

In this section, we describe the detailed routing flow. As shown in Figure 2, the inputs to the router are LEF, DEF and guide files. LEF and DEF files are industry-standard formats. The route guide file serves as the global routing solution. Given the inputs, we first set up the design database. Next, we take several data preparation steps. Then, we perform track assignment, multiple iterations of detailed routing and output a routed DEF.

### A. Data preparation

The data preparation step processes the design database to generate assisting structures, including via ordering, guide processing, region query, DRC LUT generation and pin access analysis.

1) *Via ordering*: Via ordering is the step to select default viadef(s) used for pin access and detailed routing. We sort all viadefs according to (i) number of cuts; (ii) default via property; (iii) enclosure direction; (iv) enclosure area; and (v) enclosure width. In detailed routing, we only use the minimal-enclosed default single-cut viadef, with both lower and upper-layer enclosure along the preferred routing direction. In pin access analysis, in addition to the viadef we use in detail routing, we also use the minimal-enclosed default single-cut viadef, with the lower-layer enclosure orthogonal to the preferred routing direction, and the upper-layer enclosure along the preferred routing direction. Overall, we select one of two viadefs to access the pin, and only use one viadef for all other connections. Figure 5 illustrates the ordered viadefs for detailed routing, additional viadef for pin access analysis, and a non-preferred viadef.<sup>3</sup>

<sup>2</sup>In our work, we derive the GCell size based on global routing solution, in the “route guide” format of ISPD18, ISPD19 and ICCAD19 contests. GR solutions in practice (to our knowledge) commonly use  $\sim 15$  M2 tracks as a typical GCell dimension.

<sup>3</sup>Ultimately, the via ordering step should be replaced with a more robust via generation and LEF matching strategy in a future work.

![Flowchart of the overall routing flow. The process starts with inputs LEF, DEF, and Guide leading to Database Setup. This is followed by a Routing Data Preparation block containing Via Ordering, Region Query, Guide Processing, and DRC LUT Generation. Next is Pin Access Analysis with Access Point Generation. Then Track Assignment. The main loop is Detailed Routing, which starts with Iteration 0 containing Worker 0, Worker 1, Worker 2, and Worker 3. Each worker performs Initialization, Routing, and Writeback. This is followed by Iteration 1, Iteration 2, and Iteration 3. The process ends with Routed DEF.](37a6ab1d23efb9dc00cfae09d353b1da_img.jpg)

```

graph TD
    LEF((LEF)) --> DB[Database Setup]
    DEF((DEF)) --> DB
    Guide((Guide)) --> DB
    DB --> RDP[Routing Data Preparation]
    subgraph RDP
        VO[Via Ordering]
        RQ[Region Query]
        GP[Guide Processing]
        DLG[DRC LUT Generation]
    end
    RDP --> PAA[Pin Access Analysis]
    subgraph PAA
        APG[Access Point Generation]
    end
    PAA --> TA[Track Assignment]
    TA --> DR[Detailed Routing]
    subgraph DR
        I0[Iteration 0]
        subgraph I0
            W0[Worker 0]
            W1[Worker 1]
            W2[Worker 2]
            W3[Worker 3]
            subgraph W0
                                IW0[Initialization] --> RW0[Routing] --> WW0[Writeback]
                            end
                            subgraph W1
                                IW1[Initialization] --> RW1[Routing] --> WW1[Writeback]
                            end
                            subgraph W2
                                IW2[Initialization] --> RW2[Routing] --> WW2[Writeback]
                            end
                            subgraph W3
                                IW3[Initialization] --> RW3[Routing] --> WW3[Writeback]
                            end
                        end
                        I1[Iteration 1]
                        I2[Iteration 2]
                        I3[Iteration 3]
                        Dots1[⋮]
                    end
                    I3 --> Dots2[⋮]
                    Dots2 --> RD[Routed DEF]
    end
    
```

Flowchart of the overall routing flow. The process starts with inputs LEF, DEF, and Guide leading to Database Setup. This is followed by a Routing Data Preparation block containing Via Ordering, Region Query, Guide Processing, and DRC LUT Generation. Next is Pin Access Analysis with Access Point Generation. Then Track Assignment. The main loop is Detailed Routing, which starts with Iteration 0 containing Worker 0, Worker 1, Worker 2, and Worker 3. Each worker performs Initialization, Routing, and Writeback. This is followed by Iteration 1, Iteration 2, and Iteration 3. The process ends with Routed DEF.

Fig. 2: Overall flow.

2) *Guide processing*: Guide processing [7][18] is the step to transform a set of input route guides into a standardized tree-like global routing solution.<sup>4</sup> A route guide specifies a rectangular region on a specific metal layer. A global routing solution for a net may contain several route guides on some or all of the metal layers. If we abstract the guide by drawing a center line for each guide along the preferred routing direction, we take the center lines to form a connected graph, as shown in Figure 3(e).

To standardize on a guide dimension that is conducive to form a trimmed tree-like global routing solution, we first extract the most common offset and width of all guides to form GCELLGRIDS [7], then process all route guides with **splitting**, **merging** and **bridging** techniques. Given the input guides in Figure 3(a), we first split the guide according to the

<sup>4</sup>Ultimately, the solution quality of detailed routing may be improved with an input of a better global routing solution that satisfies our guide processing behavior in a future work.

![Figure 3: Preprocessing steps for route guides. (a) Initial route guides on a grid with pins A and B. (b) Splitting guides into segments. (c) Merging touching segments. (d) Bridging abutting segments. (e) Preprocessed guides with GCELLGRID and center lines. Legend: Guide (M1, vertical) is blue, Guide (M2, horizontal) is red, Pin is a black dot, GCELLGRID is a dotted line, Center line (global routing solution) is a dashed line.](690fce4fb5c9cbb8beb560cb2a3fcbeb_img.jpg)

Figure 3: Preprocessing steps for route guides. (a) Initial route guides on a grid with pins A and B. (b) Splitting guides into segments. (c) Merging touching segments. (d) Bridging abutting segments. (e) Preprocessed guides with GCELLGRID and center lines. Legend: Guide (M1, vertical) is blue, Guide (M2, horizontal) is red, Pin is a black dot, GCELLGRID is a dotted line, Center line (global routing solution) is a dashed line.

Fig. 3: Preprocessing: (a) initial route guides; (b) splitting; (c) merging; (d) bridging and (e) preprocessed guides. The preferred direction for M1 is vertical, and for M2 is horizontal.

![Figure 4: DRC LUT illustrations. (a) via to jog (vertical); (b) via to jog (horizontal); (c) via to via (vertical); (d) via to via (horizontal); (e) jog to jog (vertical); and (f) jog to jog (horizontal). Legend: Default-width routing is black, Via enclosure is a hatched rectangle.](aa81b9b80bd1e3d723922b3a033564a2_img.jpg)

Figure 4: DRC LUT illustrations. (a) via to jog (vertical); (b) via to jog (horizontal); (c) via to via (vertical); (d) via to via (horizontal); (e) jog to jog (vertical); and (f) jog to jog (horizontal). Legend: Default-width routing is black, Via enclosure is a hatched rectangle.

Fig. 4: DRC LUT: (a) via to jog (vertical); (b) via to jog (horizontal); (c) via to via (vertical); (d) via to via (horizontal); (e) jog to jog (vertical); and (f) jog to jog (horizontal).

![Figure 5: Ordered viadefs. (a) preferred viadef for detailed routing; (b) additional viadef for pin access analysis; and (c) non-preferred viadef. Legend: Metal1 (H) is blue, Cut1 is yellow, Metal2 (V) is orange, Preferred direction track is a dashed line.](f76f0f5dfff2511b51f6b875867c029f_img.jpg)

Figure 5: Ordered viadefs. (a) preferred viadef for detailed routing; (b) additional viadef for pin access analysis; and (c) non-preferred viadef. Legend: Metal1 (H) is blue, Cut1 is yellow, Metal2 (V) is orange, Preferred direction track is a dashed line.

Fig. 5: Illustrations of ordered viadefs: (a) preferred viadef for detailed routing; (b) additional viadef for pin access analysis; and (c) non-preferred viadef.

GCELLGRID along the preferred routing direction for each metal layer, as shown in Figure 3(b); we then merge touching guides along the preferred routing direction, as shown in Figure 3(c). Last, for abutting guides along the non-preferred routing direction, we bridge them by creating upper-layer (or, otherwise, lower-layer) guides, as shown in Figure 3(d).

The above procedures guarantee a connected global routing solution as long as the input guides satisfy the assumption described in [7]. To remove redundant edges (i.e., loops) in a global routing solution, we further perform  $A^*$  search from any pin to all other pins through the processed guides. All off-path guides are removed.

3) *Region Query*: Region query is the data structure for fast shape queries. The input to the region query engine is a bounding box on a specific layer. The outputs are all intersecting shapes, in the form of  $\{\text{bbox}, \text{owner}\}$  pairs. For polygon shapes, we decompose the polygon into rectangles to be used in the region query engine. The owner belongs to one of the following types: term, instTerm, blockage, instBlockage, pathSeg, via or patchMetal.

4) *LUT Generation*: LUT (lookup table) generation is the step to construct assisting data structure to avoid same-net design rule check violations. In grid-based path search, we use object cost (described in Section V-B) to avoid potential DRCs to existing objects. To prevent DRCs within the current

path, i.e., same-net violation, we characterize the minimum default-width routing length between any two-object pair of an up via, a down via and a jog, on all metal layers, and in all directions. Figure 4 illustrates three types of minimum length requirement: via to jog, via to via, and jog to jog, in both  $x$  and  $y$  directions. In our implementation, we characterize separately for the up via and down via. In grid-based path search, we apply additional cost if the minimum length between vias and / or jogs is not satisfied.

5) *Pin access analysis*: For each pin, we generate at least  $K$  access points ( $K = 3$  in our implementation) using the pin access analysis methodology from [19]. An access point is an  $x$ - $y$  coordinate on a metal layer where the detailed router ends routing. Each access point stores from which direction the router can access the pin. There are six access directions: west, east, south, north, up and down. For the planar four directions, we check whether a wire can be used to access the pin DRC-free. For the up direction, we check whether the first two vias according to the via ordering can be used to access the pin DRC-free. We do not check or use the down access direction in this work. Each access point may indicate multiple valid access directions. For the up direction, we also store which vias are valid to use, among which one via is primary (preferred to use). The access point must be on the pin shape.

### B. Track assignment

We adopt a simplified version of greedy track assignment [33]. To reduce the problem size and lay a foundation for future parallel implementation, we perform the track assignment every 50 GCell panels. Each GCell panel has length along the preferred routing direction and spans 50 GCell heights. The initial track assignment is applied once on all horizontal layers, then on all vertical layers. According to [33], we then perform one iteration of track reassignment to optimize the solution quality.

![Figure 6: Grid graph. (a) preferred-direction grid lines on Metal1; (b) preferred-direction grid lines on Metal2; (c) preferred-direction grid lines on Metal3; and (d) overlay of grid lines (3D grid graph projected onto the x-y plane).](191a4a245a7d36d03be9a990d0f758f5_img.jpg)

Figure 6 shows four diagrams (a, b, c, d) illustrating the grid graph construction. (a) shows horizontal preferred-direction grid lines on Metal1, with a pin access point on an off-track y-coordinate. (b) shows vertical preferred-direction grid lines on Metal2, with an off-track grid on the left boundary. (c) shows horizontal preferred-direction grid lines on Metal3. (d) shows the overlay of all grid lines from Metal1, Metal2, and Metal3, forming a 3D grid graph projected onto the x-y plane. A legend on the right identifies Metal1 (H) as blue, Metal2 (V) as orange, Metal3 (H) as green, Pin access point (Metal1) as a blue dot, On-track grid (preferred direction) as a solid line, and Off-track grid (preferred direction) as a dashed line.

Figure 6: Grid graph. (a) preferred-direction grid lines on Metal1; (b) preferred-direction grid lines on Metal2; (c) preferred-direction grid lines on Metal3; and (d) overlay of grid lines (3D grid graph projected onto the x-y plane).

Fig. 6: Grid graph: (a) preferred-direction grid lines on Metal1; (b) preferred-direction grid lines on Metal2; (c) preferred-direction grid lines on Metal3; and (d) overlay of grid lines (3D grid graph projected onto the  $x$ - $y$  plane).

### C. Detailed routing

Given the track assignment result, we perform multiple iterations of detailed routing. In each iteration, we partition the design into  $7 \times 7$ , non-overlapping GCell-aligned clips, and create one **detailed routing worker** for each clip. Each detailed routing worker first initializes its own data structures (worker database) from the global database, then performs routing and design rule checking, all without touching the global database. Last, each worker commits the changes by writing back to the global database. In alternate iterations, we shift the partitioning of  $7 \times 7$  clips with an offset of 0 and -4 to enable optimization at clip boundaries. We describe the detailed routing flow inside the detailed routing worker in Section V.

In the construction of a detailed routing worker, each clip comes with three bounding boxes: **standard**, **DRC** and **extended box**. The standard box is the above-mentioned  $7 \times 7$ , non-overlapping GCell-aligned clip. The detailed routing worker can only modify objects with their center lines on or within the standard box. The DRC box is slightly larger than the standard box, enclosing the bounding box of all modifiable objects. We only count and writeback those markers intersecting with the DRC box. The extended box is slightly larger than the DRC box, allowing design rule check across the DRC box. In the detailed routing worker database, all objects within the extended box are constructed locally. Only the objects that are on or within the standard box are modifiable, while other objects are fixed. The fixed objects are used for cost calculation and design rule checking.

## V. DETAILED ROUTING WORKER

In this section, we describe the methodology to perform gridded,  $A^*$ -based detailed routing inside the detailed routing worker. We first describe the grid graph structure and various types of costs. Then, we describe the overall ripup-and-reroute flow of a detailed routing worker. Last, we detail the methodology to route one net.

### A. Grid graph

The grid graph is an essential part of detailed routing because the path search algorithm works directly on the grid graph, and various costs and properties are associated with the grid vertices and edges in the grid graph. In TritonRoute, we build a non-regular-spaced 3D grid graph supporting **irregular tracks** and **off-track routing**.

1) *Construction:* We now describe how to generate the preferred-direction grid lines on each metal layer. We first form all grid lines that are on-track – i.e., align with the DEF TRACKS definitions. Then we form all grid lines that are off-track – i.e., the center lines along the preferred direction for any existing pathSegs, vias and pin access points. We also form the grid lines on the boundary. We do not generate the grid lines in the non-preferred direction. However, bi-directional routing is still available as described in Section V-A2.

Figure 6 shows how we form the grid lines. Figure 6(a) shows horizontal Metal1, with 7 regular-spaced tracks from DEF. The Metal1 pin has an access point with an off-track y-coordinate. Thus, we create an off-track grid line according to the pin access point location. Figure 6(b) shows vertical Metal2, with 5 regular-spaced tracks from DEF. We additionally create an off-track grid on the left boundary. By always creating grid lines along the boundaries of the routing region, we make sure that at least one path exists in the grid graph in any direction, in the case that no on- and off-track grid lines exist (e.g., given a small routing region). Since the center line of the Metal1 pin access point aligns with a Metal2 track, we do not build additional off-track grid lines on Metal2. Similarly, we build grid lines on Metal3. Note that Metal1 and Metal3 grid lines do not necessarily align.

In Figure 6(d), we show the overlay of  $x$ - and  $y$ -direction grid lines. The grid vertices are formed by intersecting all  $x$ - and  $y$ -direction grid lines, and repeating  $|Z|$  times along the  $z$  direction. Each vertex has six neighbors (except the boundary vertices) – west, east, south, north, down and up; this is the 3D grid (projected into the  $x$ - $y$  plane) that we use in TritonRoute.

TABLE IV: Edge properties.

| Type    | Name        | Meaning                                        |
|---------|-------------|------------------------------------------------|
| boolean | isEnable    | whether the edge exists in path search         |
| boolean | isOnTrack   | whether the edge is on track                   |
| boolean | isOnPrefDir | whether the edge is on the preferred direction |
| viadef  | specialVia  | special via                                    |
| int     | objCost     | object cost                                    |
| int     | markerCost  | marker cost                                    |

TABLE V: Vertex properties.

| Type    | Name    | Meaning                             |
|---------|---------|-------------------------------------|
| enum    | prevDir | incoming direction                  |
| boolean | isSrc   | whether the vertex is a source      |
| boolean | isDst   | whether the vertex is a destination |

2) *Edge:* The edge properties are summarized in Table IV. As shown in Figure 6, not every grid line exists in every metal layer. We use *isEnable* to show whether the edge exists in the path search. A planar edge in the preferred direction is enabled if it is on a current layer grid line. A planar edge in the non-

preferred direction is enabled if it is on an upper-layer grid line (if any, otherwise lower-layer). Via edges are enabled between any two preferred-direction grid lines on neighboring metal layers. For each edge, we use *isOnTrack* to show whether the edge is on track; we use *isOnPrefDir* to show whether the edge is on the preferred direction. For a via edge, *specialVia* indicates whether the router should choose a special via instead of the default via. Only pin access points may have this special via property. We preprocess and mark relevant via edges for all up-via pin accesses (using non-default via). There are two types of costs associated with each edge, object cost and marker cost. We describe these costs in Section V-B.

3) *Vertex*: The vertex properties are summarized in Table V. In A\*-based path search, after a path is found, we only know the ending vertex. We use *prevDir* to indicate the incoming direction of the current vertex so that we are able to trace back the path. We use *isSrc* (resp. *isDst*) to indicate whether the vertex is a source (resp. destination).

### B. Routing cost

We use two types of costs: **object cost**, and **marker cost**. Overall, object cost is applied around an existing shape. This cost preemptively guides the path search to go around existing objects to avoid potential DRCs. The marker cost is applied around an existing DRC marker. In the ripup-and-reroute scheme, this cost helps the nets to be routed avoiding the DRC hotspots given the history of DRC data.

1) *Object cost*: Object cost is the cost originated from an object, and stored in neighboring edges to the object. We modify this cost whenever the worker database adds or removes an object, e.g., at the time of database initialization, after net ripup, or after routing of one net. We use the object cost to prevent potential design rule check violations. The evaluation of object cost is non-precise but quick, and does not invoke the DRC engine.<sup>5</sup> We support three types of spacing rules for object cost: (i) SPACINGTABLE PARALLELRUNLENGTH; (ii) SPACING ENDOFLINE; and (iii) SPACING (cut).

For parallel run length spacing, given a **target object**, we first draw an expanding region in which objects on the intersecting edges may cause DRCs, as shown in Figure 7(a). The expanding region extends beyond the target object up to the maximum required spacing plus half the default width for planar edges, and half the via enclosure for via edges. We then assume a **shadow object** (either a default-width pathSeg or a via) on each of the neighboring planar and via edges, and check against the target object, as shown in Figure 7(b). For a pathSeg on a planar edge, since the exact length of the shadow object can be arbitrarily longer than the edge length, we add pessimism by assuming maximum parallel run length between the two objects to accelerate convergence. The maximum parallel run length is the length of the target object regardless of the actual parallel run length. For each via edge,

we assume a default via, or the special via stored with the edge, and check the via enclosure against the target object. The parallel run length between a shadow via enclosure and the target object is calculated by their actual parallel run length. We modify the cost of the edge if there is a violation. Here, the modification of the costs also helps to avoid short violations since the expansion region implicitly includes those edges that may have potential short violations with the target object.

![Figure 7: Object cost from parallel run length spacing. (a) shows an expanding region around a target object (black rectangle) on a grid graph. The region extends horizontally by 'spacing + ext'. (b) shows a shadow object (grey rectangle) on a neighboring edge, with a dashed arrow indicating the parallel run length between the target object and the shadow object. A legend on the right identifies the grid graph, metal shape 1 (black), metal shape 2 (grey), and the expanding region (hatched).](9791722d75115ddcc599b07d7bc35d73_img.jpg)

Figure 7: Object cost from parallel run length spacing. (a) shows an expanding region around a target object (black rectangle) on a grid graph. The region extends horizontally by 'spacing + ext'. (b) shows a shadow object (grey rectangle) on a neighboring edge, with a dashed arrow indicating the parallel run length between the target object and the shadow object. A legend on the right identifies the grid graph, metal shape 1 (black), metal shape 2 (grey), and the expanding region (hatched).

Fig. 7: Object cost from parallel run length spacing: (a) expanding region; and (b) shadow object.

For end-of-line spacing, we only check the target object if it is a via, and the spacing is only checked along the preferred routing direction of the metal layer. Spacing orthogonal to the preferred routing direction is not checked to avoid pessimism since almost all jogs end with a preferred-direction routing or a default via, making the line end a non-end-of-line edge. Figure 8 illustrates the procedure.

![Figure 8: Object cost from end-of-line spacing. (a) shows an expanding region around a target object (black rectangle) on a grid graph. The region extends horizontally by 'eolWithin + ext'. (b) shows a shadow object (grey rectangle) on a neighboring edge, with a dashed arrow indicating the preferred routing direction (horizontal) from the target object to the shadow object. A legend on the right identifies the grid graph, target object (black), shadow object (grey), and the expanding region (hatched).](c9d8a18a6137ad054b841d7a614afb48_img.jpg)

Figure 8: Object cost from end-of-line spacing. (a) shows an expanding region around a target object (black rectangle) on a grid graph. The region extends horizontally by 'eolWithin + ext'. (b) shows a shadow object (grey rectangle) on a neighboring edge, with a dashed arrow indicating the preferred routing direction (horizontal) from the target object to the shadow object. A legend on the right identifies the grid graph, target object (black), shadow object (grey), and the expanding region (hatched).

Fig. 8: Object cost from end-of-line spacing: (a) expanding region and (b) shadow object. The preferred routing direction is horizontal.

For cut spacing, given a target via, we check all neighboring via edges which could potentially cause a cut spacing violation. For each via edge, we assume a default via (or the special via stored with the edge) and check against the target via. We modify the cost of the via edge if there is a violation.

The object cost has no history. For example, an object cost is added to the neighboring edges of the target object after the object is created, and subtracted from the neighboring edges of the target object after the object is removed. The object cost calculation supports same-net overriding, blockage spacing overriding and other exceptions. For more details pertaining to this and other parts of our discussion, please refer to [37].

2) *Marker cost*: Marker cost is the cost applied according to the DRC markers after each call to the DRC engine. For each marker, we get all objects touching the marker, and add costs to the nearest edge(s) that are used to form the objects. The marker cost has history within the detailed routing worker. For example, a marker cost is added to an edge and decayed over time (*currIter* in Algorithm 1), but is never subtracted due to the removal of a specific marker. Here, marker cost history only persists within the detailed routing worker. There is no history between detailed routing iterations shown in Figure 2.

<sup>5</sup>We do not have a metric for “precision” of object cost evaluation. The goals of the quick object cost evaluation, in decreasing priority order, are: (i) quickness, and (ii) help avoidance of repeated cycles of violations (e.g., arising due to DRC marker cost in A\* search). In practice, we see that our use of quick object cost evaluation – which naturally must be pessimistic – helps avoid cycling.

### C. Routing flow

Now we describe the routing flow inside a detailed routing worker. In Algorithm 1, Line 2 first initializes the worker database from the global database. In this step, we construct a local netlist from the connectivity of routing objects. Figure 9 shows an example, where a single net passes through the standard box twice, with two parts disjoint. In this case, we construct two subnets so that ripup-and-reroute does not change the connectivity of the net.

![Figure 9: Local netlist construction. A diagram showing a 'Standard box' (dotted rectangle) containing two disjoint subnets, 'Subnet1' (hatched) and 'Subnet2' (cross-hatched). A 'Net' (solid black) is shown passing through the box, with its parts connected to the subnets. A legend on the right identifies the symbols: dotted line for Standard box, solid black for Net, hatched for Subnet1, and cross-hatched for Subnet2.](d0abac95583b52a3b35f74a215567334_img.jpg)

Figure 9: Local netlist construction. A diagram showing a 'Standard box' (dotted rectangle) containing two disjoint subnets, 'Subnet1' (hatched) and 'Subnet2' (cross-hatched). A 'Net' (solid black) is shown passing through the box, with its parts connected to the subnets. A legend on the right identifies the symbols: dotted line for Standard box, solid black for Net, hatched for Subnet1, and cross-hatched for Subnet2.

Fig. 9: Local netlist construction: two disjoint subnets constructed in the detailed routing worker from one global net.

In Lines 3 – 20, we perform up to  $maxIter$  iterations of ripup-and-rerouting.<sup>6</sup> In each iteration, we ripup the problematic nets and reroute each one sequentially. Line 4 adds the marker cost according to all existing markers. Line 5 gets all nets that are associated with markers. We order the nets according to their distance to the nearest marker and route them sequentially. Line 6 rips up those nets and Line 7 subtracts the object cost from the ripped-up objects. Here, the boundary objects outside the standard box are not removed and their object costs remain. Since nets are routed sequentially, according to the net ordering, we would like to avoid the  $i^{th}$  net blocking the pin access of the  $j^{th}$  ( $j > i$ ) net. In Line 8, we reserve the pin access of all unrouted nets (ripped-up nets) by adding the object cost of their preferred pin access (an up via) as if those pin access points are used.

In Lines 9 – 15, we route each net once according to the net ordering. Before routing, Line 10 unreserves the pin access for the current net by subtracting the corresponding object cost of the preferred pin access (up via). Line 11 subtracts the object cost for the boundary objects outside the standard box to avoid unnecessary costs when we connect the net to the boundary pin. Line 12 routes the current net. Line 13 adds the object cost for all the newly routed objects. Line 14 adds back the object cost for boundary objects to prevent design rule violations between these objects to the remaining unrouted nets. Lines 16 – 19 perform design rule checking, and terminates the ripup-and-reroute flow once the clip is clean.

Line 21 commits the worker database back to the global database.

<sup>6</sup>Note that this number of iterations is different from the number of “outer” iterations in Figure 2. For the results that we report in this work, we perform seven (outer) iterations. The  $maxIter$  number of iterations in Algorithm 1 defines the maximum number of ripup-and-reroute iterations a net inside a DRWorker can undergo. In the current implementation / results represented in this paper, we use (1, 4, 4, 4, 4, 4, 4) as the  $maxIter$  (for ripup-and-reroute) for each net in the seven “outer” iterations, respectively.

#### Algorithm 1 Routing flow

```

1: Input: worker database, worker markers markers
2: WorkerDBInit()
3: while currIter < maxIter do
4:   addMarkerCost(markers)
5:   nets ← getMarkerNets(markers)
6:   ripupNets(nets)
7:   subObjCost(nets)
8:   reservePA(nets)
9:   for all net ∈ nets do
10:    unreservePinAccess(net)
11:    subBoundCost(net)
12:    routeOneNet(net)
13:    addObjCost(net)
14:    addBoundCost(net)
15:  end for
16:  DRC(nets)
17:  if numMarkers = 0 then
18:    break
19:  end if
20: end while
21: DBCommit()

```

### D. Routing one net

1) *Flow:* We now describe the methodology to route one net in a detailed routing worker. In our current implementation, in the standard box, a net is either fully routed or unrouted, but not partially routed. Algorithm 2 describes the methodology to route one net. Line 2 gets all unconnected pins, including standard box boundary pins and pins from instTerm and term. Line 3 holds the set of visited grid vertices, and we initialize the set to be empty. Lines 4 and 5 select the source pin to perform path search and remove it from the unconnected pins. To select the source pin, we first calculate the center of gravity for all pins in the  $x$ - $y$  plane, then select the pin furthest away from the center of gravity as the source. Line 6 performs the initialization described in Section V-D2. In Lines 7 – 11, we perform the path search as long as there are still unconnected pins. After path search, we update the grid graph in preparation for the next round of path search. The writeDB function backtraces the path to create the routing objects according to the path.

![Figure 10: Minimum area patch metal. (a) shows a 'pathSeg' (solid black) entering a 'Standard box' (dotted rectangle) from the left. A 'Patch metal' (cross-hatched) is added inside the box to the right of the pathSeg. (b) shows the 'pathSeg' exiting the box to the right. The 'Patch metal' is added outside the box to the right of the pathSeg. A legend on the right identifies the symbols: dotted line for Standard box, solid black for pathSeg, and cross-hatched for Patch metal.](740442c999390734911677f01af0316d_img.jpg)

Figure 10: Minimum area patch metal. (a) shows a 'pathSeg' (solid black) entering a 'Standard box' (dotted rectangle) from the left. A 'Patch metal' (cross-hatched) is added inside the box to the right of the pathSeg. (b) shows the 'pathSeg' exiting the box to the right. The 'Patch metal' is added outside the box to the right of the pathSeg. A legend on the right identifies the symbols: dotted line for Standard box, solid black for pathSeg, and cross-hatched for Patch metal.

Fig. 10: Minimum area patch metal: (a) patch metal considering area outside of standard box; and (b) patch metal always along the preferred routing direction even if the routing ends in the non-preferred direction. We assume the preferred routing direction is horizontal. We do not allow the patch metal to exceed the standard box. If there are more than one patch metal choices, e.g., adding to the left or to the right of a routing object, we choose the one with smaller object cost.

During backtracing, we calculate the total metal area and add necessary patch metals to satisfy the minimum area rule. The patch metals are always created with default routing width along the preferred routing direction. In our implementation,

we also build assisting structures to calculate necessary patch metal area for objects connected to the boundary pin. Figure 10 gives two examples of patch metal addition. The path search is completed once all pins are connected. The path search algorithm is described in Algorithm 4. The update function is described in Algorithm 5.

#### --- **Algorithm 2** Route one net ---

```

1: Input: net  $n$ , grid graph  $G$ 
2:  $unConnPins \leftarrow allPins(n)$ 
3:  $visitedGrids \leftarrow \emptyset$ 
4:  $srcPin \leftarrow selectSrcPin(unConnPins)$ 
5:  $unConnPins.removePin(srcPin)$ 
6:  $init(n, srcPin, unConnPins, visitedGrids, G)$ 
7: while not  $isEmpty(unConnPins)$  do
8:    $path \leftarrow search(visitedGrid, G)$ 
9:    $update(n, path, unConnPins, visitedGrids, G)$ 
10:   $writeDB(n, path)$ 
11: end while

```

---

2) *Initialization:* Algorithm 3 describes the initialization procedure. In Line 2, we first reset the previous direction flag for each grid vertex. In Lines 3 – 6, we set the source flag for all vertices on the access points of the source pin, and add the vertices to the visited grids. In Lines 7 – 11, we set the destination flag for all vertices on the access points of all destination pins. After initialization of the grid graph, the core path search algorithm does not need to look for objects and properties of the net, which is beneficial to the runtime.

#### --- **Algorithm 3** Initialization ---

```

1: Input:  $n$ ,  $srcPin$ ,  $unConnPins$ ,  $visitedGrids$ ,  $G$ 
2:  $G.resetPrevDir()$ 
3: for all  $grid \in srcPin$  do
4:    $G.setSrc(grid)$ 
5:    $visitedGrids.add(grid)$ 
6: end for
7: for all  $dstPin \in unConnPins$  do
8:   for all  $grid \in dstPin$  do
9:      $G.setDst(grid)$ 
10:  end for
11: end for

```

---

3) *Path search:* Algorithm 4 details the path search. The A\*-based path search is based on a priority queue. Each element in the priority queue is an element of the search’s wavefront, representing that a path exists from the source up to the wavefront grid vertex. In Lines 3 – 5, we first push all visited grids (source) to the queue as the initial wavefront vertices. Then in Lines 6 – 16, we pop the wavefront vertex with the least cost. We use the previous direction to indicate whether the wavefront vertex has been visited before. Lines 9 – 11 skip the wavefront vertex if it has been visited before. In Lines 12 – 14, we check whether the wavefront vertex is the destination, and return the path when reaching the destination. Otherwise, we expand the wavefront vertex by pushing its neighbors into the priority queue (with proper cost) as new wavefront vertices.

Here, the cost in the priority queue is the A\* cost, consisting of an existing path cost and an estimated future cost, as shown in Equation (1). Whenever we expand from a wavefront vertex to its neighboring vertex, the existing cost is the cost from the wavefront vertex plus the cost to its neighbor, as shown

#### --- **Algorithm 4** Search ---

```

1: Input:  $visitedGrids$ ,  $G$ 
2: Initialize  $wf$ 
3: for all  $grid \in visitedGrids$  do
4:    $wf.push(grid)$ 
5: end for
6: while not  $isEmpty(wf)$  do
7:    $currGrid \leftarrow wf.top()$ 
8:    $wf.pop()$ 
9:   if  $hasPrevDir(currGrid)$  then
10:    continue
11:   end if
12:   if  $isDst(currGrid)$  then
13:     return path
14:   else
15:      $expand(currGrid)$ 
16:   end if
17: end while

```

---

in Equation (2). The cost is the sum of edge length, plus  $8 \times$  edge length if the edge has a non-zero object cost, and  $64 \times$  edge length if the edge has a non-zero marker cost. In addition, we apply a penalty  $p$  if any match to the DRC LUT is found. The estimated future cost is the Manhattan distance to a pre-determined destination, as shown in Equation (3). If there are more than one unconnected pins to be connected, the pre-determined destination is the bounding box of the unconnected pin that is the closest to the bounding box of all visited grids. The Manhattan distance in  $z$  direction (between two neighboring metal layers) is calculated as  $4 \times$  the lower metal layer pitch.

$$cost_{tot} = cost_{wf} + cost_{est} \quad (1)$$

$$cost_{wf} = cost_{wf} + len_e + objCost_e + markerCost_e + p \quad (2)$$

$$cost_{est} = dist_{wf, dst} \quad (3)$$

As described in Lines 9 – 11, we avoid expanding an already-visited vertex by checking its previous directional flag. In an ideal A\*-based path search with a consistent path cost and a lower-bounded estimated future cost, each vertex only needs at most one visit to get the minimum cost path. However, considering the inconsistent nature of the penalty applied from the DRC LUT, the worst-case complexity of A\*-based path search becomes  $O(n^2)$ . To balance the tradeoff between runtime and solution quality, we write the previous direction to a vertex only after two more wavefront expansions are performed from that vertex.

4) *Update:* Algorithm 5 describes the methodology to update the grid graph. In Line 2, we reset the previous direction flag for every grid vertex in preparation of the next path search. In Lines 3 – 6, we set the source flag for every grid vertex along the path. We then add these grid vertices to the visited grids. Here the source flag and the visited grids serve the same purpose as they both identify the new sources for the next round of path search. However, visited grids are stored in a vector-like container to allow us to initialize the wavefront for the next path search in batches. In Lines 7 – 15, we identify the destination pin that we route to in the current round of path search, remove it from the unconnected pins, and reset the destination flag on all access points of the destination pin.

We now describe two special cases for **pin feedthrough**. Pin feedthrough describes a scenario where two (or multiple) parts of the net are connected to different access points of the same pin. We can either enable, or disable pin feedthrough. Disabling pin feedthrough forces that only one access point per pin can be used.

In case of enabling feedthrough, all access points of the destination pin, even those we do not route to, now become new sources for the next round of path search, as shown in Lines 12 –14.

In case of disabling feedthrough, special handling methodology is needed for the first source pin of the net, described in Lines 17 – 24. Recall that in Line 4 of Algorithm 3, we set the source flag on all access points of the source pin. Given feedthrough disabled, we must reset the source flag on all unused access points of the source pin once the first path search completes.

#### Algorithm 5 Update

---

```

1: Input:  $n$ ,  $path$ ,  $unConnPins$ ,  $visitedGrids$ ,  $G$ 
2:  $G.resetPrevDir()$ 
3: for all  $grid \in path$  do
4:    $setSrc(grid)$ 
5:    $visitedGrids \leftarrow add(grid)$ 
6: end for
7:  $endGrid \leftarrow path.end()$ 
8:  $currDstPin \leftarrow findPin(endGrid)$ 
9:  $unConnPins.removePin(currDstPin)$ 
10: for all  $grid \in currDstPin$  do
11:    $G.resetDst(grid)$ 
12:   if  $isAllowPinAsFeedThrough()$  then
13:      $G.setSrc(grid)$ 
14:   end if
15: end for
16:  $beginGrid \leftarrow path.begin()$ 
17: if not  $isAllowPinAsFeedThrough()$  then
18:   if  $findPin(beginGrid)$  then
19:      $currSrcPin \leftarrow findPin(beginGrid)$ 
20:     for all  $grid \neq beginGrid \in currSrcPin$  do
21:        $G.resetSrc(g)$ 
22:     end for
23:   end if
24: end if

```

---

## VI. EXPERIMENTS

In this section, we present experimental setup and results.

### A. Setup

We implement our router in C++ with LEF/DEF parser [40] and Boost C++ libraries [39]. We perform experiments using the ISPD-2018 benchmark suite [25]. The ISPD-2018 benchmark suite provides 10 testcases in 45nm and 32nm nodes, with up to 290K standard cells and 182K nets. These designs are industrial benchmarks – including large memory cells, off-track pin access, IO ports, and power and macro blockages – with realistic design rules offered in industry-standard input/output formats. ISPD-2018 benchmark information is summarized in Table VI.

The ISPD-2018 contest evaluation metrics consist of three components: (i) routing, including wirelength and via count; (ii) guides and tracks obedience, including out-of-guide wire and vias, off-track wire and vias, and wrong-way wire; and (iii)

DRCs, including area of metal shorts, number of minimum area violations and number of spacing violations. However, in the experimental results below, we do not report (ii), and make several improvements to (iii) according to the following.

- We do not strictly obey the guides since TritonRoute is not targeting the ISPD-2018 contest. According to the contest organizers, strict guide obedience was never their initial intention although all participating teams and the following published papers all strictly follow the route guides.
- We do not report the off-track and wrong-way routing although they are already considered throughout the routing flow. In all our reported testcases, such off-track and wrong-way routing account for 0.68% of the total wirelength on average.
- We report all types of DRCs, including all ISPD-2018 centric DRCs plus (number of) metal short, non-sufficient metal overlap and minimum width. The number of metal short is a good indicator of the strength of the detailed router. Non-sufficient metal overlap and minimum width are two design rules existing in the input, but not considered in the contest evaluation. We believe that the reporting of all types of DRCs effectively forbids any optimization targeting the contest metric.

Among all recently published academic detailed routers [13][22][31] that are capable of delivering ISPD-2018 contest solutions, Dr. CU 2.0 [22] dominates the solution quality for all ten testcases in terms of DRCs. Thus, we compare our TritonRoute to Dr. CU 2.0. All experiments are performed using a single thread on an Intel Xeon server.

### B. Results

Experimental results are shown in Table VII and Table VIII. Table VII gives wirelength, via count, memory consumption and runtime; Table VIII gives the details of DRCs.

As a prerequisite, a routing solution is valid only if there are no open nets. All of our reported solutions meet the connectivity requirement. Furthermore, our solution guarantees a loop-free and dangling wire-free solution (except the minimum area patch metals).

We achieve DRC-clean solution for ispd18\_test1, and reach an unprecedented, extremely low level of DRCs (<20) in seven of 10 testcases while consuming substantially reduced memory, with similar single-threaded runtime. This translates to a 99.3% reduction of DRCs as compared to known best detailed routing solutions from all published academic detailed routers. For the remaining three testcases, we reduce DRCs by 75.1% on average, and by 60.0% at a minimum. Overall, compared to the known best detailed routing solutions, TritonRoute improves wirelength by up to 0.8% (avg. 0.4%), via count by up to 16.1% (avg. 9.3%), and DRCs by up to 100% (avg. 92.0%). TritonRoute completes routing with smaller wirelength and smaller via count, and leaves only a fraction of DRCs compared to all other academic detailed routers.

We have also performed a case-study experiment using different standard box sizes to analyze the tradeoff between

TABLE VI: Benchmark information [25].

| Benchmark     | #std   | #blk | #net   | #pin | #layer | Die size                       | Tech. node |
|---------------|--------|------|--------|------|--------|--------------------------------|------------|
| ispd18_test1  | 8879   | 0    | 3153   | 0    | 9      | $0.20 \times 0.19 \text{mm}^2$ | 45nm       |
| ispd18_test2  | 35913  | 0    | 36834  | 1211 | 9      | $0.65 \times 0.57 \text{mm}^2$ | 45nm       |
| ispd18_test3  | 35973  | 4    | 36700  | 1211 | 9      | $0.99 \times 0.70 \text{mm}^2$ | 45nm       |
| ispd18_test4  | 72094  | 0    | 72401  | 1211 | 9      | $0.89 \times 0.61 \text{mm}^2$ | 32nm       |
| ispd18_test5  | 71954  | 0    | 72394  | 1211 | 9      | $0.93 \times 0.92 \text{mm}^2$ | 32nm       |
| ispd18_test6  | 107919 | 0    | 107701 | 1211 | 9      | $0.86 \times 0.53 \text{mm}^2$ | 32nm       |
| ispd18_test7  | 179865 | 16   | 179863 | 1211 | 9      | $1.36 \times 1.33 \text{mm}^2$ | 32nm       |
| ispd18_test8  | 191987 | 16   | 179863 | 1211 | 9      | $1.36 \times 1.33 \text{mm}^2$ | 32nm       |
| ispd18_test9  | 192911 | 0    | 178857 | 1211 | 9      | $0.91 \times 0.78 \text{mm}^2$ | 32nm       |
| ispd18_test10 | 290386 | 0    | 182000 | 1211 | 9      | $0.91 \times 0.87 \text{mm}^2$ | 32nm       |

TABLE VII: Comparison of wirelength, via count, memory usage and runtime between TritonRoute (TR) and Dr. CU (CU).

| Benchmark     | Wirelength ( $\mu\text{m}$ ) |                | Via count      |               | Memory (GB) |       | Runtime (s) |             |
|---------------|------------------------------|----------------|----------------|---------------|-------------|-------|-------------|-------------|
|               | TR                           | CU             | TR             | CU            | TR          | CU    | TR          | CU          |
| ispd18_test1  | <b>86025</b>                 | 86709          | 32912          | <b>32402</b>  | <b>0.08</b> | 0.21  | 61          | <b>40</b>   |
| ispd18_test2  | 1570651                      | <b>1566537</b> | <b>319855</b>  | 325684        | <b>0.43</b> | 1.39  | 614         | <b>578</b>  |
| ispd18_test3  | 1750028                      | <b>1743561</b> | 319456         | <b>318309</b> | <b>0.47</b> | 1.51  | 824         | <b>788</b>  |
| ispd18_test4  | <b>2620890</b>               | 2641860        | <b>695901</b>  | 729312        | <b>1.09</b> | 5.72  | <b>1866</b> | 3422        |
| ispd18_test5  | <b>2763186</b>               | 2780130        | <b>831775</b>  | 965544        | <b>1.29</b> | 4.61  | <b>1722</b> | 2383        |
| ispd18_test6  | <b>3557744</b>               | 3570351        | <b>1241673</b> | 1480617       | <b>1.71</b> | 5.72  | <b>2682</b> | 3357        |
| ispd18_test7  | <b>6482066</b>               | 6517341        | <b>2041794</b> | 2402543       | <b>3.07</b> | 9.87  | <b>5023</b> | 5847        |
| ispd18_test8  | <b>6513278</b>               | 6546908        | <b>2062997</b> | 2412121       | <b>3.11</b> | 10.47 | <b>4916</b> | 5932        |
| ispd18_test9  | <b>5442527</b>               | 5476029        | <b>2049839</b> | 2410790       | <b>2.71</b> | 10.11 | <b>4378</b> | 4910        |
| ispd18_test10 | <b>6769942</b>               | 6809019        | <b>2226243</b> | 2594386       | <b>3.09</b> | 10.58 | 10129       | <b>9380</b> |

TABLE VIII: Comparison of number of minimum width (MinWid), non-sufficient-metal overlap (NSMet), minimum area (MAR), metal short (Short), cut short (CShort), metal parallel run length spacing (MetSpc), metal end-of-line spacing (EOLSpC), cut spacing (CutSpc) and total design rule violations between TritonRoute (TR) and Dr. CU (CU).

| Benchmark     | Design rule violations |    |          |       |          |     |             |      |          |          |            |     |           |           |           |          |             |       |
|---------------|------------------------|----|----------|-------|----------|-----|-------------|------|----------|----------|------------|-----|-----------|-----------|-----------|----------|-------------|-------|
|               | #MinWid                |    | #NSMet   |       | #MAR     |     | #Short      |      | #CShort  |          | #MetSpc    |     | #EOLSpC   |           | #CutSpc   |          | #Total      |       |
|               | TR                     | CU | TR       | CU    | TR       | CU  | TR          | CU   | TR       | CU       | TR         | CU  | TR        | CU        | TR        | CU       | TR          | CU    |
| ispd18_test1  | <b>0</b>               | 0  | <b>0</b> | 1716  | <b>0</b> | 0   | <b>0</b>    | 1    | <b>0</b> | 0        | <b>0</b>   | 1   | <b>0</b>  | 1         | <b>0</b>  | 0        | <b>0</b>    | 1719  |
| ispd18_test2  | <b>0</b>               | 0  | <b>0</b> | 20048 | <b>0</b> | 0   | <b>1</b>    | 1    | <b>0</b> | 0        | <b>7</b>   | 49  | <b>9</b>  | 9         | <b>0</b>  | 0        | <b>17</b>   | 20107 |
| ispd18_test3  | <b>0</b>               | 0  | <b>0</b> | 21224 | <b>0</b> | 0   | <b>112</b>  | 219  | <b>1</b> | 0        | <b>17</b>  | 86  | <b>10</b> | <b>9</b>  | <b>2</b>  | <b>0</b> | <b>142</b>  | 21538 |
| ispd18_test4  | <b>0</b>               | 10 | <b>2</b> | 17    | <b>0</b> | 32  | <b>190</b>  | 287  | <b>0</b> | 0        | <b>132</b> | 289 | <b>2</b>  | 164       | <b>0</b>  | 142      | <b>326</b>  | 941   |
| ispd18_test5  | <b>0</b>               | 7  | <b>0</b> | 19    | <b>0</b> | 48  | <b>2</b>    | 342  | <b>0</b> | 0        | <b>0</b>   | 309 | <b>0</b>  | 36        | <b>0</b>  | 20       | <b>2</b>    | 781   |
| ispd18_test6  | <b>0</b>               | 8  | <b>0</b> | 44    | <b>3</b> | 92  | <b>1</b>    | 36   | <b>0</b> | 0        | <b>2</b>   | 489 | <b>2</b>  | 21        | <b>0</b>  | 30       | <b>8</b>    | 720   |
| ispd18_test7  | <b>0</b>               | 0  | <b>0</b> | 11    | <b>5</b> | 127 | <b>4</b>    | 604  | <b>0</b> | 0        | <b>4</b>   | 129 | <b>0</b>  | 7         | <b>0</b>  | 60       | <b>13</b>   | 938   |
| ispd18_test8  | <b>0</b>               | 0  | <b>0</b> | 19    | <b>3</b> | 138 | <b>2</b>    | 625  | <b>0</b> | 0        | <b>1</b>   | 118 | <b>0</b>  | 15        | <b>0</b>  | 59       | <b>6</b>    | 974   |
| ispd18_test9  | <b>0</b>               | 0  | <b>0</b> | 16    | <b>4</b> | 185 | <b>1</b>    | 39   | <b>0</b> | 0        | <b>0</b>   | 49  | <b>0</b>  | 7         | <b>0</b>  | 54       | <b>5</b>    | 350   |
| ispd18_test10 | <b>0</b>               | 0  | <b>0</b> | 26    | <b>4</b> | 228 | <b>1103</b> | 3180 | <b>5</b> | <b>1</b> | <b>425</b> | 742 | 144       | <b>73</b> | <b>33</b> | 100      | <b>1714</b> | 4350  |

runtime and final DRC count. We sweep the standard box size from  $3 \times 3$  to  $11 \times 11$  with a step size of 2 on the ISPD18\_test3 testcase. The specific testcase that we choose has relatively high *#violation-to-#instance* ratio, which indicates that ISPD18\_test3 is a difficult and congested design among the ISPD18 contest benchmarks. Figure 11 illustrates the tradeoff between runtime and final DRC count with different standard box sizes. We observe that a larger standard box provides a larger solution space for ripup-and-reroute for DRC fixing at the cost of longer runtime for A\* search. A standard box with size of  $7 \times 7$  GCells can achieve a decent tradeoff between runtime and final DRC count, especially for difficult designs.

## VII. CONCLUSION

In this work, we present TritonRoute, an open source detailed router. We describe an in-memory router database, and an end-to-end detailed routing scheme. We evaluate our router using the official ISPD-2018 benchmark suite, and show that we reach an unprecedented, extremely low level of DRCs ( $< 20$ ) in seven of ten testcases, a 99.3% reduction of DRCs on average compared to known best detailed routing solution

![Figure 11: A line graph titled 'ISPD18_test3' showing the tradeoff between runtime and final DRC count for various DRWorker standard box sizes. The x-axis represents 'DRWorker (standard box) size' with values 3x3, 5x5, 7x7, 9x9, and 11x11. The left y-axis represents 'Runtime (s)' ranging from 500 to 750. The right y-axis represents '#DRC' ranging from 130 to 155. A blue line represents the runtime, which increases from approximately 500s at 3x3 to about 750s at 11x11. A red line represents the DRC count, which decreases from approximately 155 at 3x3 to about 130 at 11x11. The two lines intersect at the 7x7 box size.](2f73c3f1961c12d27d0d18fe7befbf0c_img.jpg)

| DRWorker (standard box) size | Runtime (s) | #DRC |
|------------------------------|-------------|------|
| $3 \times 3$                 | ~500        | ~155 |
| $5 \times 5$                 | ~550        | ~145 |
| $7 \times 7$                 | ~600        | ~140 |
| $9 \times 9$                 | ~650        | ~135 |
| $11 \times 11$               | ~750        | ~130 |

Figure 11: A line graph titled 'ISPD18\_test3' showing the tradeoff between runtime and final DRC count for various DRWorker standard box sizes. The x-axis represents 'DRWorker (standard box) size' with values 3x3, 5x5, 7x7, 9x9, and 11x11. The left y-axis represents 'Runtime (s)' ranging from 500 to 750. The right y-axis represents '#DRC' ranging from 130 to 155. A blue line represents the runtime, which increases from approximately 500s at 3x3 to about 750s at 11x11. A red line represents the DRC count, which decreases from approximately 155 at 3x3 to about 130 at 11x11. The two lines intersect at the 7x7 box size.

Fig. 11: Illustration of tradeoff between runtime and final DRC count with various DRWorker standard box sizes in unit of GCell.

from all published academic detail routers. Overall, compared to the known best detailed routing solution, TritonRoute improves wirelength by up to 0.8% (avg. 0.4%), via count by up to 16.1% (avg. 9.3%), and DRCs by up to 100% (avg. 92.0%). Due to its generic nature, our framework can support extensions to new technologies or design rules. Our ongoing work includes: (i) support of multi-threading; (ii) track assignment improvement; (iii) runtime improvement; (iv) support of advanced technology nodes (including ISPD-2019 contest benchmarks); and (v) support of via generation and via swapping.

## VIII. ACKNOWLEDGMENTS

We thank Dr. Patrick Groeneveld, Dr. Wen-Hao Liu and Dr. Stefanus Mantik for providing valuable feedback.

## REFERENCES

- [1] M. Ahrens, M. Gester, N. Klewinghaus, D. Müller, S. Peyer, C. Schulte and G. Tellez, "Detailed Routing Algorithms for Advanced Technology Nodes", *IEEE Trans. on CAD* 34(4) (2015), pp. 563-576.
- [2] F.-Y. Chang, R.-S. Tsay, W.-K. Mak and S.-H. Chen, "MANA: A Shortest Path Maze Algorithm Under Separation and Minimum Length Nanometer Rules", *IEEE Trans. on CAD* 32(10) (2013), pp. 1557-1568.
- [3] H.-Y. Chen and Y.-W. Chang, "Global and Detailed Routing", Chapter 12 in Wang, Chang and Cheng (Eds.), *Electronic Design Automation: Synthesis, Verification, and Test*, Morgan Kaufmann, 2009, pp. 687-749. [http://cc.ee.ntu.edu.tw/~ywcang/Courses/PD\\_Source/EDA\\_routing.pdf](http://cc.ee.ntu.edu.tw/~ywcang/Courses/PD_Source/EDA_routing.pdf)
- [4] G. Chen, C.-W. Pui, H. Li, J. Chen, B. Jiang and E. F. Y. Young, "Detailed Routing by Sparse Grid Graph and Minimum-Area-Captured Path Search", *Proc. ASP-DAC*, 2019, pp. 754-760.
- [5] G. Chen, C.-W. Pui, H. Li and E. F. Y. Young, "Dr. CU: Detailed Routing by Sparse Grid Graph and Minimum-Area-Captured Path Search", *IEEE Trans. on CAD*, to appear. DOI: 10.1109/TCAD.2019.2927542
- [6] Y. Ding, C. Chu and W.-K. Mak, "Self-Aligned Double Patterning Lithography Aware Detailed Routing with Color Preassignment", *IEEE Trans. on CAD* 36(8) (2017), pp. 1381-1394.
- [7] S. Dolgov, A. Volkov, L. Wang and B. Xu, "2019 CAD Contest: LEF/DEF Based Global Routing", *Proc. ICCAD*, 2019, to appear.
- [8] Y. Du, Q. Ma, H. Song, J. Shiely, G. Luk-Pat, A. Miloslavsky and M. D. F. Wong, "Spacer-is-Dielectric-Compliant Detailed Routing for Self-Aligned Double Patterning Lithography", *Proc. DAC*, 2013, pp. 1-6.
- [9] A. Feller, "Automatic Layout of Low-Cost Quick-Turnaround Random-Logic Custom LSI Devices", *Proc. DAC*, 1976, pp. 79-85.
- [10] G.-R. Gao and D. Z. Pan, "Flexible Self-Aligned Double Patterning Aware Detailed Routing with Prescribed Layout Planning", *Proc. ISPD*, 2012, pp. 25-32.
- [11] M. Gester, D. Müller, T. Nieberg, C. Panten, C. Schulte and J. Vygen, "BonnRoute: Algorithms and Data Structures for Fast and Good VLSI Routing", *ACM TODAES* 18(2) (2013), pp. 32:1-32:24.
- [12] S. M. M. Gonçalves, L. S. da Rosa and F. de S. Marques, "An Improved Heuristic Function for A\*-Based Path Search in Detailed Routing", *Proc. ISCAS*, 2019, pp. 1-5.
- [13] S. M. M. Gonçalves, L. S. da Rosa and F. de S. Marques, "DRAPS: A Design Rule Aware Path Search Algorithm for Detailed Routing", *IEEE Trans. on Circuits and Systems II: Express Briefs* (2019).
- [14] F. O. Hadlock, "A Shortest Path Algorithm for Grid Graphs", *Networks* 7(4) (1977), pp. 323-334.
- [15] K. Han, A. B. Kahng and H. Lee, "Evaluation of BEOL Design Rule Impacts Using an Optimal ILP-Based Detailed Router", *Proc. DAC*, 2015, pp. 68:1-68:6.
- [16] A. Hetzel, "A Sequential Detailed Router for Huge Grid Graphs", *Proc. DATE*, 1998, pp. 332-339.
- [17] D. W. Hightower, "A Solution to Line-Routing Problems on the Continuous Plane", *Proc. DAC*, 1969, pp. 1-24.
- [18] A. B. Kahng, L. Wang and B. Xu, "TritonRoute: An Initial Detailed Router for Advanced VLSI Technologies", *Proc. ICCAD*, 2018, pp. 81:1-81:8.
- [19] A. B. Kahng, L. Wang and B. Xu, "The Tao of PAO: Anatomy of a Pin Access Oracle for Detailed Routing", *Proc. DAC*, 2020, to appear.
- [20] C. Y. Lee, "An Algorithm for Path Connections and Its Applications", *IRE Trans. on Electro. Comp.* 10(3) (1961), pp. 346-365.
- [21] H. K.-S. Leung, "Advanced Routing in Changing Technology Landscape", *Proc. ISPD*, 2003, pp. 118-121.
- [22] H. Li, G. Chen, B. Jiang, J. Chen and E. F. Y. Young, "Dr. CU 2.0: A Scalable Detailed Routing Framework with Correct-by-Construction Design Rule Satisfaction", *Proc. ICCAD*, 2019, pp. 1-7.
- [23] I.-J. Liu, S.-Y. Fang and Y.-W. Chang, "Overlay-Aware Detailed Routing for Self-Aligned Double Patterning Lithography Using the Cut Process", *IEEE Trans. on CAD* 35(9) (2016), pp. 1519-1531.
- [24] W. K. Luk, "A Greedy Switch-Box Router", *Integration: The VLSI Journal* 3(2) (1985), pp. 129-149.
- [25] S. Mantik, G. Posser, W.-K. Chow, Y. Ding and W.-H. Liu, "ISPD 2018 Initial Detailed Routing Contest and Benchmarks", *Proc. ISPD*, 2018, pp. 140-143.
- [26] T. Nieberg, "Gridless Pin Access in Detailed Routing", *Proc. DAC*, 2011, pp. 170-175.
- [27] N. J. Nilsson, "State-Space Search Methods", in *Problem-Solving Methods in Artificial Intelligence*, McGraw-Hill Book Co., 1971, pp. 43-79.
- [28] I. Pohl, "Bi-Directional Search", *Machine Intelligence* (1971), pp. 127-140.
- [29] E. Shragowitz and S. Keel, "A Global Router Based on a Multicommodity Flow Model", *Integration: The VLSI Journal* 5(1) (1987), pp. 3-16.
- [30] J. Soukup, "Fast Maze Router", *Proc. DAC*, 1978, pp. 100-102.
- [31] F.-K. Sun, H. Chen, C.-Y. Chen, C.-H. Hsu and Y.-W. Chang, "A Multithreaded Initial Detailed Routing Algorithm Considering Global Routing Guides", *Proc. ICCAD*, 2018, pp. 82:1-82:7.
- [32] P.-S. Tzeng, and C. H. Sequin, "Codar: A Congestion-Directed General Area Router", *Proc. ICCAD*, 1988, pp. 30-33.
- [33] M.-P. Wong, W.-H. Liu and T.-C. Wang, "Negotiation-Based Track Assignment Considering Local Nets", *Proc. ASP-DAC*, 2016, pp. 378-383.
- [34] X. Xu, B. Yu, J.-R. Gao, C.-L. Hsu and D. Z. Pan, "PARR: Pin Access Planning and Regular Routing for Self-Aligned Double Patterning", *ACM Trans. on DAES* 21(3) (2016), article 42.
- [35] Y. Zhang and C. Chu, "RegularRoute: An Efficient Detailed Router Applying Regular Routing Patterns", *IEEE Trans. on VLSI* 21(9) (2013), pp. 1655-1668.
- [36] LEF/DEF Language Reference, <http://www.ispd.cc/contests/18/lefdefref.pdf>
- [37] The-OpenROAD-Project/TritonRoute: UCSD Detailed Router, <https://github.com/The-OpenROAD-Project/TritonRoute>
- [38] W.-H. Liu, "ISPD 2018 Initial Detailed Routing Contest and Benchmarks" *presentation slides*, [http://www.ispd.cc/slides/2018/s7\\_3.pdf](http://www.ispd.cc/slides/2018/s7_3.pdf)
- [39] B. Schälting, *The Boost C++ Libraries, 2nd ed.*, XML Press, 2014.
- [40] LEF/DEF reference 5.7. <http://www.si2.org/openeda.si2.org/projects/lefdefnew>
- [41] Si2 OpenAccess. <http://projects.si2.org/?page=69>

![Portrait of Andrew B. Kahng, a man in a suit and glasses.](e159e9f78612406820a4d40e26e01413_img.jpg)

Portrait of Andrew B. Kahng, a man in a suit and glasses.

**Andrew B. Kahng** is a professor in the Computer Science Engineering Department and in the Electrical and Computer Engineering Department of the University of California at San Diego. His interests include IC physical design, the design-manufacturing interface, combinatorial optimization, and technology roadmapping. He received the Ph.D. degree in Computer Science from the University of California at San Diego.

![Portrait of Lutong Wang, a man in a suit and glasses.](53298644c66fa3fca81d6eec654afec5_img.jpg)

Portrait of Lutong Wang, a man in a suit and glasses.

**Lutong Wang** received the B.S. degree in microelectronics from Tsinghua University, Beijing, China, in 2014 and the M.S. degree in electrical and computer engineering from the University of California at San Diego, La Jolla, in 2016. He is currently pursuing the Ph.D. degree at the University of California at San Diego, La Jolla. His research interests include physical design implementation and DFM methodologies.

![Portrait of Bangqi Xu, a man in a blue shirt.](1a4800dc93053fadd723da643930a0bf_img.jpg)

Portrait of Bangqi Xu, a man in a blue shirt.

**Bangqi Xu** received B.S. degree in electrical engineering from the University of Michigan, Ann Arbor, MI, USA in 2015 and the M.S. degree in electrical and computer engineering from the University of California at San Diego, La Jolla, in 2017. He is currently pursuing the Ph.D. degree at the University of California at San Diego, La Jolla. His current research interests include detailed placement, routing methodology and optimization.