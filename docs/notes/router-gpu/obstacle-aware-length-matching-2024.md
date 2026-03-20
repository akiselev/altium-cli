

# Obstacle-Aware Length-Matching Routing for Any-Direction Traces in Printed Circuit Board

Weijie Fang, Longkun Guo\*, Jiawei Lin, Silu Xiong, Huan He, Jiacen Xu, and Jianli Chen,

**Abstract**—Emerging applications in Printed Circuit Board (PCB) routing impose new challenges on automatic length matching, including adaptability for any-direction traces with their original routing preserved for interactivity. The challenges can be addressed through two orthogonal stages: assign non-overlapping routing regions to each trace and meander the traces within their regions to reach the target length. In this paper, mainly focusing on the meandering stage, we propose an obstacle-aware detailed routing approach to optimize the utilization of available space and achieve length matching while maintaining the original routing of traces. Furthermore, our approach incorporating the proposed Multi-Scale Dynamic Time Warping (MSDTW) method can also handle differential pairs against common decoupled problems. Experimental results demonstrate that our approach has effective length-matching routing ability and compares favorably to previous approaches under more complicated constraints.

**Index Terms**—Length Matching, Any-Direction Trace, Differential Pair, Dynamic Programming, Obstacle-Aware Routing

## I. INTRODUCTION

SEVERAL protocols in Printed Circuit Board (PCB) designs demand some parallel signals in the same group to arrive at their destination simultaneously. Mismatching the arrival timing of these signals may result in a critical clock skew that harms the stability and functionality of designs, which derives length-matching techniques to minimize the difference between their propagation delay by matching the length of their traces. Many current length-matching tools still require manual assistance to resolve detail routing in obstacle-dense regions. Meanwhile, length-matching approaches for high-speed designs need to fit routing in any direction, including but not limited to the traditional routing in  $90^\circ$  or  $135^\circ$ , which is illustrated in Fig. 1.

### A. Related Works

Much research has been conducted on length matching in recent years, while various approaches have proven their effectiveness in tackling different specific difficulties.

In the aspect of obstacle awareness, many existing methods use a gridded strategy to previously fix safety tracks that do not intersect with obstacles and then determine the detailed

![Figure 1: Illustration of routing in various directions and the primary distances restricted in DRC. The diagram shows a trace starting from the left, labeled 'any-direction routing' in red, which then transitions to '90° routing' in green. It then encounters an obstacle (solid black polygon) and must navigate around it. The trace then transitions to '135° routing' in blue. Several distances are indicated: d_obs (obstacle distance), d_miter-3 (miter distance), d_gap (gap distance), and d_protect (protect distance).](d3294dc879b451b369c0b06f42e9b39f_img.jpg)

Figure 1: Illustration of routing in various directions and the primary distances restricted in DRC. The diagram shows a trace starting from the left, labeled 'any-direction routing' in red, which then transitions to '90° routing' in green. It then encounters an obstacle (solid black polygon) and must navigate around it. The trace then transitions to '135° routing' in blue. Several distances are indicated: d\_obs (obstacle distance), d\_miter-3 (miter distance), d\_gap (gap distance), and d\_protect (protect distance).

Fig. 1. Illustration of routing in various directions and the primary distances restricted in DRC. Solid polygons in the figure denote obstacles.

routing of traces on these tracks. Kohira et al. employed a gridded approach based on the biconnected component to evaluate the upper bound of meandering in space with obstacles and proposed a routing algorithm to approximate this upper bound [4]. Then, they further proposed a heuristic Connectivity Aware Frontier Exploration (CAFE) router that can achieve a small error of length matching in most cases [5]. Yan et al. [6] applied an obstacle-aware region division method on gridded space and used a shortest routing path generator for length matching. Hsu et al. [16] and Chen et al. [17] presented several approaches for the clustering, connection, and rip-up and rerouting of bus routers under the obstacle environment. Cheng et al. [18] optimized routability and trace length in their obstacle-avoiding bus length matching approach. Yan et al. [3] presented a single-layer obstacle-aware bus router that can minimize the used gridded space and satisfy the length-matching constraints.

For the determination of a better length matching target. Kubo et al. [1] approached length matching with a symmetric slant grid interconnect scheme to prevent a too-large target length. Nakatani et al. [11] was dedicated to optimizing raw trace before length matching by employing a minimum cost maximum flow algorithm to reduce the maximum trace length while keeping the minimum total trace length. Ozdal et al. presented an extra resource distribution scheme during the original routing stage to support the possible following length-matching [2]. Also, they introduced Lagrangian Relaxation into length matching that attempts to allocate grid cells to traces with different priorities and minimize the target length [7]. Kito et al. [14] introduced simulated annealing into length matching to minimize trace length increment during meandering. Zhang et al. [10] used virtual boundary to fix pins and divided and processed routing space separately, resulting in the reduction of total trace length and a higher similarity among trace routing compared with a previous method [23].

Yan et al. [8] introduced Bounded-Sliceline Grid (BSG) [24] that converts length matching into a quadratic programming problem, followed by a pattern generating rule to achieve

Weijie Fang, Longkun Guo, and Jiawei Lin are with the School of Mathematics and Statistics, Fuzhou University, Fuzhou, Fujian, China.

Silu Xiong and Huan He are with Hangzhou Huawei Enterprises Telecommunication Technologies Co., Ltd, Hangzhou, Zhejiang, China.

Jiacen Xu is with Shanghai LEDA Technology Co., Ltd, Shanghai, China.

Jianli Chen is with the School of Microelectronics, Fudan University, Shanghai, China.

A preliminary version of this paper has been accepted by DAC 2024. The corresponding author is Longkun Guo (lkguo@fzu.edu.cn).

the final meandering. Tseng et al. [13] chose Integer Linear Programming (ILP) to solve length-matching problems, which set the gap between patterns as large as possible to reduce the influence of crosstalk. Based on the maximum common subsequence of the disordered pin-pairs, Zhang et al. [12] adopted a single commodity flow algorithm [25] considering the shortest path to resolve the original routing and employed R-flip and C-flip [4] to adjust trace length. Sato et al. [15] presented a pattern generator for set-pair routing by selecting and connecting pin-pairs that improve the efficiency of length matching. Lee et al. [9] studied a mature simultaneous escape routing algorithm [26] and combined it with the length matching of differential pairs based on min-cost median points [27].

### B. Motivations

Most existing works on automatic length matching may override the original routing of traces or assume traces are routed in up to eight directions. However, many high-speed PCBs nowadays are designed with traces routed in any direction, and it is usually specified to route such any-direction traces. Leading industrial commercial tools, like Allegro PCB Designer [22], specially implement a route offset function to generate such traces.

In applications, the routing of these traces is hoped to be preserved after length matching because users do not prefer a result that confuses their recognition and interaction, or seriously corrupts their previous specific routing during the Computer-Aided Design (CAD) process. Besides, a trace usually passes different Design Rule Areas (DRA), demanding the length matching approaches to consider multiple Design Rules Checking (DRC). These gaps motivate length-matching techniques to keep up with the industrial standard.

### C. Contributions

The length-matching process can be divided into two orthogonal stages: assigning non-overlapping regions for original traces and meandering each trace within its own region. This paper mainly focuses on the second stage to achieve automatic length matching while preserving their original properties as much as possible. Meanwhile, applications of length matching frequently involve differential pairs. A differential pair is commonly regarded as a wide single-ended trace during length matching, but this scheme meets many difficulties in practice, especially when the differential pair is not strictly coupled. This paper proposed the Multi-Scale Dynamic Time Warping (MSDTW) method to help tackle these difficulties. Fig. 2 illustrates the algorithmic flow of our approach.

Our contributions are summarized as follows:

- To the best of our knowledge, this paper is the first length-matching work with respect to arbitrary routing directions, and it supports obstacle-aware routing and multiple DRCs.
- The presented length-matching method combines greedy, Dynamic Programming (DP), and computational geometry. Compared with existing approaches, *it routes more flexibly without following fixed tracks or pre-defined modes relying on space regularity*, achieving length matching of any-direction traces concerning original routing.

![Flowchart of the length-matching approach. The process starts with three inputs: PCB Layout, Design Rules, and Parallel Signals. These lead to a step labeled 'Non-Overlapping Length Matching Region Assignment'. This is followed by 'Length-Matching Routing', which contains two sub-steps: 'MSDTW Differential Pairs and DRC Conversion' and a parallel block of 'DP-Based Segment Extension' and 'Best Transition Calculation'. The final output is 'Length-Matching Result'.](22b2fd4b8672ad8b02cf6cd4de5809cd_img.jpg)

```

graph TD
    A[PCB Layout] --> B[Non-Overlapping Length Matching Region Assignment]
    C[Design Rules] --> B
    D[Parallel Signals] --> B
    B --> E[Length-Matching Routing]
    subgraph E [Length-Matching Routing]
        E1[MSDTW Differential Pairs and DRC Conversion] --> E2[DP-Based Segment Extension]
        E1 --> E3[Best Transition Calculation]
    end
    E --> F[Length-Matching Result]
  
```

Flowchart of the length-matching approach. The process starts with three inputs: PCB Layout, Design Rules, and Parallel Signals. These lead to a step labeled 'Non-Overlapping Length Matching Region Assignment'. This is followed by 'Length-Matching Routing', which contains two sub-steps: 'MSDTW Differential Pairs and DRC Conversion' and a parallel block of 'DP-Based Segment Extension' and 'Best Transition Calculation'. The final output is 'Length-Matching Result'.

Fig. 2. Overview of our length-matching approach.

- We proposed the MSDTW method to facilitate the length matching among differential pairs. *It can convert a differential pair into a median trace against several issues in coupling*, and the median trace after length matching can be simply restored to the differential pair. The applications of MSDTW are not limited to our length-matching method. The remainder of this paper is organized as follows. Section 2 introduces the preliminaries of our works. Section 3 briefly discusses our region assignment. Section 4 presents our detailed meandering and MSDTW method. Section 5 demonstrates the experimental results. Section 6 concludes this paper.

## II. PROBLEM FORMULATION

Length matching is also known as delay tuning because it generally works on the traces already routed in a PCB, and the length of a trace stands for the propagation delay of the signals on it. Although propagation delay is not the only factor to be considered in timing engines, the other delays are generally ignored when discussing length matching. Nevertheless, our approach meanders each trace independently, thereby supporting the individual target lengths of each trace. Rigorously, the influence of other delays can also be considered by adjusting the target length of each trace, i.e., the precise propagation delay that each signal actually needs.

In this paper, we focus on length-matching routing on any-direction traces to meet the emerging industrial requirement from high-speed PCBs nowadays. The clarification of primary distances restricted in DRC about length matching shown in Fig. 1 is listed as follows:

- $d_{gap}$ : restricts the distance between traces to prevent self-inductance, crosstalk, etc.
- $d_{obs}$ : restricts the distance between a trace and an obstacle.
- $d_{protect}$ : restricts the minimum length to prevent the occurrence of extremely short trace segments.
- $d_{miter}$ : configures the corners mitered for convex patterns. In practice, any rotation of a right angle or an acute angle will be mitered by obtuse angles.

Some other important concepts mentioned in this paper are given as follows:

**Trace:** trace of a signal consisting of connected segments in PCB layout, also indicated by net or wire.

**Any-direction:** the traces that can be routed not only in  $90^\circ$  or  $135^\circ$  are called any-direction traces.

**Target length  $l_{target}$ :** a length that a trace in a matching group needs to match, no less than the original length of the trace.

**Routable area:** the union of non-overlapping routing regions assigned to a trace, represented as some irregular polygons.

**Obstacle:** a polygon that the trace cannot pass, converted into a part of the routable area in this paper.

Therefore, the problem we address in this paper is formulated as follows:

**Any-direction length-matching problem:** Given a PCB layout, design rules, and matching groups. For each matching group with  $l_{target}$ , extend each trace in the group utilizing the space of its routable area to make its length equal  $l_{target}$ , while preserving its original specific routing as much as possible.

For digestibility, we use the convex pattern with corners at a right angle in the remaining discussion to omit tedious details of geometry computation.

## III. REGION ASSIGNMENT

Based on the relation between length and space revealed in [8], we need only assign sufficient regions for each trace to hold feasible length-matching routing. Similar problems have been discussed in many works, such as [8] using Quadratic Programming and [7] using Lagrangian relaxation. For the sake of better fitting our specific requirement, we divide the design according to its layout to compose several regions and consider the following constraints:

- 1) Neighbor Validity: A region can only be assigned to its neighbor traces:

$$x_{ij} = 0, \text{ region } i \text{ is not the neighbor of trace } j \quad (1)$$

where  $x_{ij}$  denotes the space region  $i$  assign to trace  $j$ .

- 2) Feasibility: Any assignment of a region space must be positive and bounded by its capacity:

$$\sum_j x_{ij} \leq Cap_i, x_{ij} \geq 0, \forall i, j \quad (2)$$

where  $Cap_i$  denotes the space capacity of region  $i$ .

- 3) Sufficiency: A trace must receive sufficient space from its neighbor regions:

$$\sum_i x_{ij} \geq Req_j, \forall i, j \quad (3)$$

where  $Req_j$  denotes the required space for trace  $j$ .

Here, we employ a Linear Programming (LP) problem to solve this assignment:

Assignment Problem:

$$\begin{aligned} & \text{find:} && \text{feasible } x_{ij} \\ & \text{satisfying:} && \text{neighbor validity constraint (1)} \\ & && \text{feasibility constraint (2)} \\ & && \text{sufficiency constraint (3)} \end{aligned} \quad (4)$$

This assignment scheme ensures the preserved original routing is contained in the routable area for the following stages.

Some techniques of existing works can help to figure out a better routing if the LP is infeasible [21]. We are not going to discuss them in detail here.

## IV. DP-BASED SEGMENT EXTENSION

In order to increase the length of a trace  $l_{trace}$  to its target length  $l_{target}$ , our routing method inserts convex patterns perpendicular to its segment, this process is called the extension of segments. This extension is held by computational geometry so that it fits any-direction routing. Each segment is extended as much as possible, and a segment after the extension is replaced by several new component segments for further extension if needed. The extension will be conducted iteratively until  $l_{trace}$  is within the error tolerance of  $l_{target}$ , resulting in patterns similar to the combination of Accordion and Trombone. The whole DP-based extension is presented in Alg. 1.

### A. State Transition

Our method optimizes the extension of a segment using a DP algorithm. First of all, we discretize the segment into points using a configurable step length  $l_{disc}$ . In this paper, we define  $d(a, b)$  denotes the Euclidean distance between points  $a$  and  $b$ . Formally, a segment  $AB$  with node points  $A$  and  $B$  will be discretized to a set  $U = \{u \mid u \text{ is on segment } AB\}$  of  $n$  different points, where  $u_1 = A$ ,  $u_n = B$  and  $\forall i \in [2, n-1], d(u_{i-1}, u_i) = l_{disc}$ . We may slightly increase  $d_{gap}$  and  $d_{protect}$  or adjust  $l_{disc}$  to make the former divisible by the latter.

Based on the discretization above, we define  $dp[i][dir]$  to denote the best extension result, in which the patterns are inserted within the previous  $i$  points, and the last inserted pattern is in the  $dir$  direction of the segment. Without loss of generality, we define the clockwise direction as the positive direction and the counterclockwise direction as the negative direction, represented by “1” and “-1”, respectively. We set the initial state as:

$$dp[1][-1] = dp[1][1] = 0 \quad (5)$$

During the transition, we initialize a new state by:

$$dp[i][dir] = dp[i-1][dir], i \in [2, n] \quad (6)$$

For each pattern with  $w$ -steps width, whose feet are located in  $u_{i-w}$  and  $u_i$ , respectively, we calculate its maximum valid height  $h$  and try to attach it with the best available predecessor states to obtain a better result of the current state. Thereby, we can obtain a simplified state transition equation as follows:

$$dp[i][dir] = \max(dp[i][dir], dp[i-w][\pm dir] + h), i \in [2, n] \quad (7)$$

According to the DRC introduced in Section 2, the actual predecessor states available for the state transition rather than all cases of  $dp[i-w][\pm dir]$  shall be detailed. If the current state is transited from the one with the same direction, it must keep at least  $d_{gap}$  away from the foot of any possibly existing pattern inserted previously. While for the opposite direction, the distance that must be kept is at least  $d_{protect}$ . Besides, there is a particular valid state transition in which the

#### **Algorithm 1:** DP-based segment extension.

---

**Input:** *trace* before length matching; target length  $l_{target}$ .

**Output:** *trace* after length matching.

```

1 maintaining unexpanded segments using queue  $Q$ 
2 while  $l_{trace} \neq l_{target}$  and  $|Q| \neq 0$  do
3   pop segment  $seg$  from  $Q$ , discretize  $seg$  into point
    set  $U$ 
4    $dp[1][1] = dp[1][-1] = 0$ 
5   for  $i \leftarrow 2$  to  $n$ ,  $dir$  in  $\{-1, 1\}$  do
6      $dp[i][dir] = dp[i-1][dir]$ 
7     if  $i = n$  or  $d(u_i, u_n) \geq d_{protect}$  then
8       for  $w \leftarrow 1$  to  $i$  do
9         calculate the maximum  $h$  based on
        width  $w$ 
10        calculate  $dp[i][dir]$  considering priority
11      end
12    end
13  end
14   $dir_{max} = \arg \max_{dir} dp[n][dir]$ 
15  if  $dp[n][dir_{max}] > 0$  then
16     $l_{trace} = l_{trace} + dp[n][dir_{max}]$ 
17    restore the patterns of the best result
18    push the new segments replacing  $seg$  into  $Q$ 
19  end
20 end

```

---

pattern connects to the previously inserted one or a node of the extended segment. The above four cases are illustrated in Fig. 3. Hence, we conclude the actually meaningful states for transition as follows:

$$dp[i-w][\pm dir] = \max \begin{bmatrix} dp[p_{gap}][dir] \\ dp[p_{protect}][-dir] \\ dp[p_{local}][-dir] \end{bmatrix}, \quad i \in [2, n]$$

where

$$\begin{cases} p_{gap} &= i - w - \frac{d_{gap}}{l_{disc}} \\ p_{protect} &= i - w - \frac{d_{protect}}{l_{disc}} \\ p_{local} &= i - w, \text{ need extra condition} \end{cases} \quad (8)$$

Certainly, none of these positions can be less than 1. Otherwise, the corresponding state is invalid.

If multiple predecessor states have the same value, choosing any of them will not change the final result of  $dp[i][dir]$ . However, different choices may affect the following states because the transition from  $p_{local}$  needs an extra condition illustrated in Fig. 4. Besides, without compromising the current result, the state in which two patterns are connecting will likely bring capacity for extra patterns, as illustrated in Fig. 5, which benefits further meandering on the meandered patterns in possible subsequent iterations. We retain these states as a higher priority during state transitions.

There may come a consideration: To ensure the integrity of

![Figure 3: Four kinds of valid state transitions. (a) From the same direction: shows a step function with a gap d_gap between points p_gap and i. (b) From the opposite direction: shows a step function with a protection distance d_protect between points p_protect and i. (c) Connect to a pattern: shows a step function with a point p_local and a dashed line to i. (d) Connect to a node point: shows a step function with a point p_local and a dashed line to i.](32acdaad6c921e80fd17e42562858b80_img.jpg)

Figure 3: Four kinds of valid state transitions. (a) From the same direction: shows a step function with a gap d\_gap between points p\_gap and i. (b) From the opposite direction: shows a step function with a protection distance d\_protect between points p\_protect and i. (c) Connect to a pattern: shows a step function with a point p\_local and a dashed line to i. (d) Connect to a node point: shows a step function with a point p\_local and a dashed line to i.

Fig. 3. Four kinds of valid state transitions.

![Figure 4: Illustration of different candidate states with the same value. (a) i meets the extra condition: shows a step function with a point i = p_local and a checkmark. (b) i does not meet the extra condition: shows a step function with a point i = p_local and an X mark.](f5e131a3fffe09aa98db055df84e4378_img.jpg)

Figure 4: Illustration of different candidate states with the same value. (a) i meets the extra condition: shows a step function with a point i = p\_local and a checkmark. (b) i does not meet the extra condition: shows a step function with a point i = p\_local and an X mark.

Fig. 4. Illustration of different candidate states with the same value. (a) and (b) contribute the same value to  $dp[i][dir]$ . Only (a) allows the transition of  $p_{local}$ , so it has a higher priority than (b) to be retained.

state transition, is it necessary to maintain both sub-states of  $dp[i][dir]$  for cases without and with a new pattern whose one foot is inserted in the  $i$ th point? Let us assume  $dp[i][dir][0]$  and  $dp[i][dir][1]$  denote the above two cases respectively without loss of generality, then Eq. (7) will be converted to:

$$\begin{cases} dp[i][dir][0] = \max \begin{bmatrix} dp[i-1][dir][0] \\ dp[i-1][dir][1] \end{bmatrix} \\ dp[i][dir][1] = \max \begin{bmatrix} dp[i-w][\pm dir][0] + h \\ dp[i-w][\pm dir][1] + h \end{bmatrix} \end{cases}, \quad i \in [2, n]$$

where

$$\begin{cases} dp[i-w][\pm dir][0] = \max \begin{bmatrix} dp[p_{gap}][dir][0] \\ dp[p_{protect}][-dir][0] \end{bmatrix} \\ dp[i-w][\pm dir][1] = \max \begin{bmatrix} dp[p_{gap}][dir][1] \\ dp[p_{protect}][-dir][1] \\ dp[p_{local}][-dir][1] \end{bmatrix} \end{cases} \quad (9)$$

Observing, except the state  $dp[p_{local}][-dir][0]$  is never used because of the invalid transition, the other cases of  $dp[i][dir][0]$  and  $dp[i][dir][1]$  can be merged into  $dp[i][dir]$  since we always choose the maximum one of them before adding an identical  $h$ . And this merge can be implemented automatically by DP itself if we do not adopt the last dimension to maintain both of the sub-states. For handling the exception of  $p_{local}$ , we mark each state for whether it is transited through a newly inserted pattern, and this mark is also useful in restoring the final result.

![Figure 5: Illustration of why the patterns are hoped to be connected. (a) Connected: A path with a single node (black dot) on a horizontal segment. (b) Disconnected: A path with two nodes (black dots) on a horizontal segment, with a vertical segment between them.](690fce4fb5c9cbb8beb560cb2a3fcbeb_img.jpg)

Figure 5: Illustration of why the patterns are hoped to be connected. (a) Connected: A path with a single node (black dot) on a horizontal segment. (b) Disconnected: A path with two nodes (black dots) on a horizontal segment, with a vertical segment between them.

Fig. 5. Illustration of why the patterns are hoped to be connected. The original segment has a capacity of only two patterns, so both cases have the same DP result. However, the former case can provide the capacity of an extra pattern.

### B. Maximum Transition Gain of DP

To determine the maximum transition gain of the proposed DP, we need to calculate the maximum valid height  $h$  of pattern  $C$  built on  $u_{i-w}, u_i$ . Notably, that  $h$  is valid does not guarantee any height  $h' \leq h$  if the pattern routes around obstacles because a shrunk pattern may intersect with some obstacles that used to lay inside it, so monotonicity-based methods like binary search cannot be adopted to calculate the genuine  $h$ . Instead, we first create  $C$  with the height equal to the remaining extension requirement and then shrink  $h$  until all violations of DRC are eliminated. Multiple DRAs will be separated into independent rouTable areas and handled independently.

Here, we give the concept of UnReachable Area (URA): The URA of a segment is a rectangle whose border is half of  $d_{gap}$  away from the segment, and the URA of a pattern is the union of three segments' URAs, as illustrated in Fig. 6. Therefore, we convert DRC into intersection checking between the polygons that stand for URAs or the rouTable area. Even though the URAs of the previous patterns in the current DP are uncertain, they can be ignored since the validation of state transitions has considered DRC. For convenience, we call edges  $AB$  and  $CD$  the “sides” and  $BC$  the “hat” of URA. The area below line  $AD$  need not be checked because the URA of the original segment certainly lies there, so no other polygons shall exist. During shrinking, we update the height of the outer border  $h_{ob}$  of URA and calculate  $h$  as:

$$h = \max \left( 0, h_{ob} - \frac{d_{gap}}{2} \right) \quad (10)$$

The shrinking of  $h$  begins with eliminating the violation of DRC for its outer border. In the first place, we shrink  $h_{ob}$  according to the intersection of “sides” with other polygons. In this paper, we define the distance between a point  $p$  and the extended segment  $seg$  as  $d(seg, p)$ , and the distance between

![Figure 6: Illustration of URA. A polygon ABCD is shown with a dashed line representing the URA. The distance between the segment BC and the dashed line is labeled 0.5d_gap. The polygon ABCD is called its outer border, and EFGH is called its inner border.](d980a3f9608055996a07f31788baf827_img.jpg)

Figure 6: Illustration of URA. A polygon ABCD is shown with a dashed line representing the URA. The distance between the segment BC and the dashed line is labeled 0.5d\_gap. The polygon ABCD is called its outer border, and EFGH is called its inner border.

Fig. 6. Illustration of URA. Dash segments represent the URA of a pattern.  $ABCD$  is called its outer border,  $EFGH$  is called its inner border.

#### Algorithm 2: Shrinking by checking node position.

---

**Input:**  $\{Poly_k\}$ ; Initial URA and  $h_{ob}$ .  
**Output:** Final  $P_{inside}$ ;  $h_{ob}$  after shrinking.

---

```

1  $P_{check} = \{p \mid x_p \in [x_A, x_C], y_p \in [y_D, y_B]\}$ 
2 figure out initial  $P_{inside}$  based on  $P_{check}$ 
3 while true do
4   foreach  $k$  do
5     update  $Poly_k^{in}$ 
6     if  $0 < |Poly_k^{in}| < |Poly_k|$  then
7        $h_{ob} = \min(h_{ob}, d(seg, Poly_k^{in}))$ 
8        $P_{inside} = P_{inside} \setminus Poly_k^{in}$ 
9     end
10  end
11  if  $P_{inside} = \bigcup_k Poly_k^{in}$  then
12    break
13  end
14  establish the new outer border using  $h_{ob}$ 
15  further reduce  $P_{inside}$  according to new outer border
16 end

```

---

![Figure 7: Illustration of shrinking according to “hat”. A polygon ABCD is shown with a dashed line representing the URA. The polygon BC is shrunk iteratively to B'C' until all intersected polygons are entirely outside the outer border.](84a01685710d24f113b18758ed3c6fcb_img.jpg)

Figure 7: Illustration of shrinking according to “hat”. A polygon ABCD is shown with a dashed line representing the URA. The polygon BC is shrunk iteratively to B'C' until all intersected polygons are entirely outside the outer border.

Fig. 7. Illustration of shrinking according to “hat”. For conciseness, we only draw the outer border of URA.  $BC$  is shrunk iteratively to  $B''C''$  until all intersected polygons are entirely outside the outer border.

a point set  $P$  and  $seg$  as  $d(seg, P) = \min_{p \in P} d(seg, p)$ . Then the initial shrinking of  $h_{ob}$  based on “side” is calculated as follows:

$$h_{ob}^0 = \min(d(A, B), d(seg, P_{inters})) \quad (11)$$

where  $P_{inters}$  is the set of all intersection points mentioned above.

This way, the violations of DRC caused by the outer border are reduced to the intersection of “sides” with other polygons. Shrinking according to “hat” may be iterative, as illustrated in Fig. 7, because the shrunk border may lead to new intersections with other polygons. Observing that there must be at least one node point of each intersected polygon shall remain inside the outer border since the intersection with “sides” has been done, we derive an efficient method to solve this problem by checking whether a polygon has node points both inside and outside the outer border, which is presented in Alg. 2.

We classify all node points of polygon  $k$  into set  $Poly_k$ . Defining  $P_{inside} = \{p \mid p \text{ is inside the outer border}\}$  and the subset of  $Poly_k$  named  $Poly_k^{in} = \{p \mid p \in Poly_k \cap P_{inside}\}$ , then conditions  $|Poly_k^{in}| = 0$  and  $|Poly_k^{in}| = |Poly_k|$  can

![Figure 8: Illustration of the shrinking with the inner border. The diagram shows two stages of a polygon shrinking process. On the left, an outer boundary is shown with black dashed lines and an inner boundary with blue dashed lines. A red polygon is inside the inner boundary. The outer boundary is being shrunk iteratively (BC to B'C' to B''C'') until no polygons lie between the inner and outer borders. The right side shows the final state where the outer boundary has been shrunk to B''C''.](afe5eb459b7c9cfe880b067777d876d8_img.jpg)

Figure 8: Illustration of the shrinking with the inner border. The diagram shows two stages of a polygon shrinking process. On the left, an outer boundary is shown with black dashed lines and an inner boundary with blue dashed lines. A red polygon is inside the inner boundary. The outer boundary is being shrunk iteratively (BC to B'C' to B''C'') until no polygons lie between the inner and outer borders. The right side shows the final state where the outer boundary has been shrunk to B''C''.

Fig. 8. Illustration of the shrinking with the inner border. Black dash segments represent the outer border and its shrinking. Blue dash segments represent the inner border.  $BC$  is shrunk iteratively to  $B''C''$  until no polygons lay between the inner and outer border.

indicate the whole polygon  $k$  is outside or inside the outer border, respectively, and  $h_{ob}$  is updated iteratively as follows:

$$h_{ob}^{i+1} = \min \left( h_{ob}^i, \min_{k: 0 < |Poly_k^{in}| < |Poly_k|} d(seg, Poly_k^{in}) \right) \quad (12)$$

We build a segment tree to reduce the checking range from all node points, so that for each URA, we can quickly find a smaller point set  $P_{check} = \{p \mid x_p \in [x_A, x_C], y_p \in [y_D, y_B]\}$ , defining  $x_p$  and  $y_p$  as the coordinates of  $p$ , and figure out the initial  $P_{inside}$ . After each iteration,  $P_{inside}$  is reduced to at least  $P_{inside} \setminus Poly_k$  for all  $k$  with  $0 < |Poly_k^{in}| < |Poly_k|$ , and the iterating ends when  $P_{inside}$  is no longer reduced.

For surrounded obstacles, the last work is to check whether polygons inside the outer border intersect the inner border. Similarly, we define the subset of  $Poly_k$  named  $Poly_k^{out} = \{p \mid p \text{ is outside the inner border, } p \in Poly_k\}$ . When there exists  $Poly_k^{out} \cap P_{inside} \neq \emptyset$ ,  $h_{ob}$  must be shrunk to avoid the whole polygon  $k$  as follows:

$$h_{ob}^{i+1} = \min \left( h_{ob}^i, \min_{k: |Poly_k^{out}| > 0} d(seg, Poly_k) \right) \quad (13)$$

This shrinking is also iterative, as shown in Fig. 8. The difference from the previous one is that  $P_{inside}$  is reduced to  $P_{inside} \setminus Poly_k$  for all  $k$  with  $|Poly_k^{out}| > 0$  here.

Compared to some meandering methods [8], [21], our shrinking scheme provides a switchable function to build patterns that route around obstacles if a better state transition is met. Compared to some re-route obstacle-aware methods [3], [16], our algorithm just slightly changes the topology so that it prevents overriding the original routing.

### C. Pattern Restoration

After the DP, we choose the best state between  $dp[n][1]$  and  $dp[n][-1]$  as the extended result. Although  $dp[i][dir]$  only maintains the value of best extension results, each state has a determined final transition path, so we can easily backtrack it and restore the position of patterns in the best solution.

To this end, we employ a vector corresponding to each state to record the details about how it is obtained at last, which is represented as follows:

$$transit[i][dir] : < i', dir', w' > \quad (14)$$

where  $i'$  and  $dir'$  indicate that the state of  $dp[i][dir]$  is transited from the state of  $dp[i'][dir']$ , and  $w'$  denotes that there is a inserted pattern in  $dp[i][dir]$  whose feet are located in  $u_{i-w'}$  and  $u_i$ . Additionally,  $w' = 0$  marks that this state is

not transited through a newly inserted pattern, which is also used to check the extra condition of  $p_{local}$  during the state transitions process.

### D. Complexity Discussion

The time complexity of DP state transition is obviously  $O(n^2)$ , depending on how much the segment is discretized.

The time complexity of pattern restoration is  $O(n)$ , as we can immediately know the height and width of the pattern to be restored for  $dp[i][dir]$  using the information of  $transit[i][dir]$ , and the length of the transition path is at most  $n$ .

Suppose  $N$  is the total number of node points belonging to the borders of the rouTable area and the other URAs than which of the current extended segment. As each point is the node of a polygon, its degree must be 2 so that  $N$  also equals the number of segments existing in these polygons. Therefore, the shrinking with “sides” is with a time complexity of

$$O(N) * T(I) \quad (15)$$

where  $T(I)$  is the time complexity of calculating segment intersection, which can be regarded as  $O(1)$  without involving variables.

The time complexity of shrinking with “hat” and shrinking with the inner border shall be discussed together. We employ a segment tree to maintain points whose abscissa rank is within intervals, and the points in each tree node are sorted by ordinate. The space complexity of this segment tree is  $O(N \log_2 N)$  owing to the fact that each point appears at most  $\log_2 N$  times. To initialize  $P_{check}$ , we use abscissa range  $[x_A, x_C]$  to conduct a query in the tree and then use binary search to locate the start position of target nodes stored sequential, resulting in a time complexity of  $O(4 * (\log_2 N + \log_2 N)) = O(\log_2 N)$ .

Suppose  $M_r$  and  $M_u$  are the numbers of node points in  $P_{check}$  belonging to the borders of the rouTable area and the other URAs, respectively,  $M_r$  and  $M_u$  will generally be much less than  $N$ . Define  $K$  as the number of sets those  $N$  points are classified into. As each iteration will remove at least one of the  $K$  sets from  $P_{check}$ , and all node points belonging to the other URAs are sure to be removed after the first iteration, the following node position checking is with a time complexity of

$$O(\log_2 N) + O(KM_r + M_u) * T(R) \quad (16)$$

where  $T(R)$  is the time complexity of checking whether a point is inside the rectangular inner border. We adopt the ray casting algorithm for this work, whose time complexity can be regarded as  $O(1)$  without involving variables.

## V. MULTI-SCALE DYNAMIC TIME WARPING

A matching group may have both single-ended traces and differential pairs. During the length matching of such groups, we need not only to match the lengths of all traces, but also to keep the coupling of differential pairs. A common method of length matching involving differential pairs is to regard each differential pair as a wide single-ended trace bounded by its sub-traces. However, this trick still meets many problems in practice because the sub-traces of an actual legal differential pair may frequently not be perfectly coupled in geometry,

which may result from not strictly parallel, tiny patterns, or different passed DRAs. Fig. 9 provides a particular example from a real-world design.

To tackle these issues, we proposed MSDTW based on the Dynamic Time Warping (DTW) algorithm [19], [20], which converts a differential pair and its DRC into a median single-ended trace.

### A. Considering Nodes instead of Segments

The conventional method usually uses parallel checking to detect coupled segments of sub-traces so that each pair of coupled segments can be merged into a segment and compose the part of the median trace. This method is effective in theory, but coupled segments of sub-traces may not always be strictly parallel, as illustrated in Fig. 10. Fig. 10a shows that several short segments appear at a corner because several nodes lie. This case may happen because the coordination of the ideal position can not be represented by the precision of the machine, which makes the ideal nodes be replaced by several approximate nodes. Or it may caused by manual adjustment aiming at avoiding some other objects. Fig. 10b shows a tiny pattern used for length matching between sub-traces. The pattern causes segments  $AB$  and  $CD$  not to parallel with the segment of the other sub-trace, and segment  $BC$  will further lead the expected median segment shift from its proper position. These two cases are both common in real-world designs.

Although these cases make the coupling imperfect, the differential pair can still be legal in DRC and retained directly. However, these cases bring massive trouble in implementing the parallel checking algorithm, as the implementation must consider a lot of extraordinary case judgments. Even so, it is hard to ensure all possible cases in the future have been covered. From another perspective, we can rely on node matching instead of parallel checking to detect the coupling of segments. Whereas the segments may have some issues in the alignment of angles, the position and clustering of their nodes will not change seriously. Hence, we employ DTW to obtain the optimal node matching between sub-traces except the preserved breakout part. The matching is to minimize the total cost of all matched pairs. The method also allows multiple nodes to match the same node while promising that every node will be matched, which excels at handling the inconsistent number of nodes in two sub-traces, as shown in Fig. 10a.

Let  $trace_P$  and  $trace_N$  respectively symbolize the sub-traces in a differential pair. Without loss of generality, we define  $C[i][j]$  as the minimum cost of matching the previous  $i$  nodes of  $trace_P$  and the previous  $j$  nodes of  $trace_N$ .

![Figure 9: A photograph of a printed circuit board (PCB) showing several green differential pair traces. Some traces exhibit irregular, non-parallel behavior, illustrating decoupled differential pairs in a real-world design.](f3e03accc76df483950e65a9fb19c20e_img.jpg)

Figure 9: A photograph of a printed circuit board (PCB) showing several green differential pair traces. Some traces exhibit irregular, non-parallel behavior, illustrating decoupled differential pairs in a real-world design.

Fig. 9. An example of decoupled differential pairs in the real world.

![Figure 10: Two diagrams illustrating coupled sub-traces. (a) 'Short segments': Shows two sub-traces meeting at a corner. The top trace has nodes A, B, C, and the bottom trace has nodes D, E. Red dashed lines indicate node matching between A and D, B and E, and C and E. (b) 'Tiny pattern': Shows two sub-traces with a small pattern. The top trace has nodes A, B, C, D. The bottom trace has a cyan line with nodes. Cyan lines indicate the expected median segments.](48d61f7cf40bacce4d63f9e98ea225fb_img.jpg)

Figure 10: Two diagrams illustrating coupled sub-traces. (a) 'Short segments': Shows two sub-traces meeting at a corner. The top trace has nodes A, B, C, and the bottom trace has nodes D, E. Red dashed lines indicate node matching between A and D, B and E, and C and E. (b) 'Tiny pattern': Shows two sub-traces with a small pattern. The top trace has nodes A, B, C, D. The bottom trace has a cyan line with nodes. Cyan lines indicate the expected median segments.

Fig. 10. Illustration of coupled sub-traces whose segments are not always strictly parallel. (a) Several short segments appear at a corner. Red dash lines indicate the matching of nodes when DTW is employed. (b) A tiny pattern exists on one of the sub-traces. Cyan lines indicate the expected median segments.

Initializing state  $C[0][0] = 0$ , the state transition equation of matching is as follows:

$$C[i][j] = \min \begin{bmatrix} C[i-1][j] \\ C[i][j-1] \\ C[i-1][j-1] \end{bmatrix} + d(i, j), \quad \begin{cases} i \in [1, I] \\ j \in [1, J] \end{cases} \quad (17)$$

where  $d(i, j)$  is the distance between the  $i$ th node of  $trace_P$  and the  $j$ th node of  $trace_N$ , which denotes the matching cost, and  $I$  and  $J$  denote the number of nodes in the two sub-traces, respectively.

The matched pairs are restored by backtracking the state transition from  $C[I][J]$  to  $C[0][0]$ . We can directly find the current state transits from which of the three predecessor states according to the current matching cost and the minimum cost recorded in these predecessor states. After restoring all matched pairs, we connect every pair of matched nodes, thereby making all nodes compose several connected components. Defining  $V_C$  denotes the set of nodes in a connected component, and then  $V_C^P = \{v \mid v \in V_C, v \text{ is in } trace_P\}$ ,  $V_C^N = \{v \mid v \in V_C, v \text{ is in } trace_N\}$ , we use each  $V_C$  to generate a median point  $p_m$  as follows:

$$p_m = \overline{\{V_C^P, V_C^N\}} \quad (18)$$

where  $\overline{X}$  is the point with the average coordinate of all points in  $X$ . These median points compose the nodes of the converted median trace. This way, we first respectively calculate the median point of nodes on each sub-trace and then use them to calculate the final median point of a connected component. So that even if multiple nodes are matched to one node, the median points will not shift to one of the sub-traces.

To guarantee the differential pair can be legally restored after length matching, we also attach a virtual DRC to its merged median trace. For a differential pair, the virtual DRC is converted from its distance rule and the original DRC of its sub-traces. Thereby, the restored differential pair will not violate the original DRC as long as the merged median trace does not violate the virtual DRC during length matching.

### B. Filtering Unpaired Nodes

Matched pairs involving nodes of tiny patterns usually cause an undesirable shifting of median points, as illustrated in Fig. 11a. The naive DTW will find a matched pair for all nodes, even including those of tiny patterns. When matched

![Figure 11(a): Illustration of matched pairs involving nodes of tiny patterns. It shows two horizontal lines with nodes E, F, G, H. A tiny pattern with nodes A, B, C, D is shown above the top line. Red dashed lines indicate matching between E and F, and G and H. A cyan dot indicates the generated median point between E and F.](2ee59e629035d641140e55f4d215b0d7_img.jpg)

Figure 11(a): Illustration of matched pairs involving nodes of tiny patterns. It shows two horizontal lines with nodes E, F, G, H. A tiny pattern with nodes A, B, C, D is shown above the top line. Red dashed lines indicate matching between E and F, and G and H. A cyan dot indicates the generated median point between E and F.

(a) Matched pairs involving nodes of tiny patterns.

![Figure 11(b): Illustration of matched pairs after filtering unpaired nodes. The tiny pattern nodes A, B, C, D are now unpaired (red dots). The main nodes E, F, G, H are paired (blue dots). Blue dotted lines indicate the legally matched pairs (E-F and G-H).](d0abac95583b52a3b35f74a215567334_img.jpg)

Figure 11(b): Illustration of matched pairs after filtering unpaired nodes. The tiny pattern nodes A, B, C, D are now unpaired (red dots). The main nodes E, F, G, H are paired (blue dots). Blue dotted lines indicate the legally matched pairs (E-F and G-H).

(b) Matched pairs after filtering unpaired nodes.

Fig. 11. Illustration of the necessity and effect of filtering unpaired nodes. (a) Red dash lines indicate the matching of nodes, and the cyan point indicates the generated median point. (b) Blue dot lines indicate the legally matched pairs. Blue points and red points indicate paired nodes and unpaired nodes, respectively.

![Figure 12(a): Illustration of the issue brought by multiple DRAs. It shows a complex graph with nodes E, F, G, H, A, B, C, D. Red dashed lines indicate possible matched pairs, including E-F, G-H, and A-H. A red 'X' is placed over the A-H match, indicating it is not controllable.](5860ad6bd2a2dd8d1ab12864b8f90f37_img.jpg)

Figure 12(a): Illustration of the issue brought by multiple DRAs. It shows a complex graph with nodes E, F, G, H, A, B, C, D. Red dashed lines indicate possible matched pairs, including E-F, G-H, and A-H. A red 'X' is placed over the A-H match, indicating it is not controllable.

(a) If the greatest distance rule is used for matching, the filtering of unpaired nodes may not be controllable.

![Figure 12(b): Illustration of how MSDTW gradually matches nodes. The same graph as in (a) is shown, but now only E-F and G-H are matched (blue dotted lines). The A-H match is removed, and a red 'X' is placed over it.](e1a0d046fbe7f28f5e93a47091851747_img.jpg)

Figure 12(b): Illustration of how MSDTW gradually matches nodes. The same graph as in (a) is shown, but now only E-F and G-H are matched (blue dotted lines). The A-H match is removed, and a red 'X' is placed over it.

(b) MSDTW gradually matches nodes and splits the differential pairs.

Fig. 12. Illustration of the issue brought by multiple DRAs and how the MSDTW method tackles it. (a) Red dash lines indicate the possible matched pairs. (b) Blue dot lines indicate the successfully matched pairs in the last round, and red dash lines indicate the possible matched pairs in this round.

pairs involve nodes of tiny patterns, the corresponding connected component will generate a seriously shifted median point according to Eq. (18). Hence, the nodes of tiny patterns shall be regarded as noise during the running of DTW.

To avoid them, defining  $cost_i$  as the matching cost of the matched pair  $pair_i$  and  $r$  as the distance rule of the differential pair, we will drop  $pair_i$  if  $cost_i > \sqrt{2}r$ . Considering the rotation angle of a trace must be obtuse, any matched pair, even if at a corner, shall meet  $cost_i > \sqrt{2}r$ , otherwise we can determine that it is a matched pair involving nodes of tiny

#### Algorithm 3: Multi-Scale Dynamic Time Warping.

**Input:** Differential pair  $df_{original}$ ; Rule set  $R$ .

**Output:** Set of all matched pairs  $M$ .

```

1  $M = \emptyset$ , set of differential sub-pairs  $S = \{df_{original}\}$ 
2 foreach  $r$  in  $R$  do
3   foreach  $df_i$  in  $S$  do
4     calculate the current matched pairs set  $M_i$ 
5     foreach  $pair_j$  in  $M_i$  do
6       if  $cost_j > \sqrt{2}r$  then
7          $M_i = M_i \setminus \{pair_j\}$ 
8       end
9     end
10     $M = M \cup M_i$ 
11    split  $df_i$  into  $S_{split}$  using matched pairs in  $M_i$ 
12    foreach  $df_{split}$  in  $S_{split}$  do
13      if no nodes in trace_P or trace_N then
14         $S_{split} = S_{split} \setminus \{df_{split}\}$ 
15      end
16    end
17     $S = S_{split} \cup S \setminus \{df_i\}$ 
18  end
19 end

```

patterns. The nodes that only belong to the dropped matched pairs are called unpaired nodes, and the remaining nodes are called paired nodes. Consequently, all unpaired nodes will be filtered and no longer influence the position of generated median points, as shown in Fig. 11b.

### C. Tackling Multiple Design Rule Areas

The filtering of unpaired nodes relies on the distance rule of differential pairs, but if a differential pair passes multiple DRAs, the corresponding multiple distance rules may cause filtering failure. As illustrated in Fig. 12a, node pair  $EF$  and  $GH$  belong to different DRAs. If we use  $d(E, F)$  as the distance rule for matching, the unpaired node  $A$  cannot be filtered since  $d(A, H) < \sqrt{2} * d(E, F)$ . And if we use  $d(A, H)$  as the distance rule, the correctness of matching in other DRAs cannot be promised.

To handle differential pairs passing multiple DRAs, our MSDTW method sequentially matches the nodes by increasing distance rules, the so-called multi-scale. Defining  $R = \{r_0, r_1, \dots, r_m\}$  as the set of all involved distance rules whose elements are arranged in increasing order, we match the nodes by enumerating the elements of  $R$  and continuously filter unpaired nodes after each round of matching.

![Figure 13: Final median trace of a differential pair merged by MSDTW. It shows a 'differential pair' (red dashed lines) and an 'original DRC' (black dashed line). The 'median trace' (blue solid line) is shown, and the 'converted DRC' (cyan dashed line) is shown below it. Red dots indicate matched pairs.](de483453104e12bc074397762fd59975_img.jpg)

Figure 13: Final median trace of a differential pair merged by MSDTW. It shows a 'differential pair' (red dashed lines) and an 'original DRC' (black dashed line). The 'median trace' (blue solid line) is shown, and the 'converted DRC' (cyan dashed line) is shown below it. Red dots indicate matched pairs.

Fig. 13. Final median trace of a differential pair merged by MSDTW. Red dash lines in the figure indicate matched pairs.

TABLE I  
LENGTH-MATCHING PERFORMANCE COMPARED WITH ALLEGRO [22] AiDT

| case | $\frac{l_{target}}{d_{gap}}$ | group size | trace type   | spacing | Max. error (%) |              |             | Avg. error (%) |         |             | runtime (s) |             |
|------|------------------------------|------------|--------------|---------|----------------|--------------|-------------|----------------|---------|-------------|-------------|-------------|
|      |                              |            |              |         | Initial        | Allegro      | Ours        | Initial        | Allegro | Ours        | Allegro     | Ours        |
| 1    | 205.88                       | 8          | single-ended | dense   | 37.38          | 33.52        | <b>3.02</b> | 19.02          | 14.23   | <b>1.30</b> | <b>0.92</b> | 6.87        |
| 2    | 199.02                       | 8          | single-ended | dense   | 35.99          | 28.06        | <b>3.93</b> | 19.41          | 11.04   | <b>1.39</b> | <b>0.78</b> | 3.98        |
| 3    | 187.25                       | 8          | single-ended | dense   | 35.91          | 20.91        | <b>3.51</b> | 20.06          | 8.66    | <b>1.37</b> | <b>0.81</b> | 5.27        |
| 4    | 186.27                       | 8          | single-ended | dense   | 30.99          | 22.25        | <b>5.46</b> | 17.22          | 9.85    | <b>1.83</b> | <b>0.72</b> | 2.86        |
| 5    | 217.32                       | 4          | differential | sparse  | 26.55          | <b>10.21</b> | 10.3        | 15.18          | 5.14    | <b>3.32</b> | 5.07        | <b>3.22</b> |

In the beginning, we match nodes by the smallest  $r_0 \in R$ , so that we can temporarily prevent the matching from involving any node of tiny patterns. Once the matching of the current round is determined, we split the remaining differential pair into differential sub-pairs according to the matched nodes in this round. If there has been no node on  $trace_P$  or  $trace_N$  of a sub-pair, this sub-pair will be dropped immediately since no more meaningful matching can occur. This dropping strategy holds because tiny patterns are used for length matching between sub-traces and shall only appear on either  $trace_P$  or  $trace_N$ , or else two tiny patterns in different sub-traces can be reduced by each other.

This way, we can filter all unpaired nodes in the DRA corresponding to  $r_0$ , thus isolating them from the matching of the subsequent rounds. In the next round, we select the second smallest  $r_1 \in R$  and match nodes individually in each retained sub-pair, which means that the matching of nodes across sub-pairs is also forbidden. As shown in Fig. 12b, by a smaller distance rule, node pair  $GH$  is successfully matched while node pair  $AH$  is dropped. Then,  $GH$  splits the original differential pair into two sub-pairs so that the matching of node pair  $AF$  is also forbidden.

This recursion ends when no sub-pair is further split, and then we collect all successfully matched pairs as the result. At this point, we can present the whole MSDTW method in Alg. 3 and its illustration in Fig. 13. After length matching, we restore the differential pairs and compensate tiny patterns to sub-traces if needed.

## VI. EXPERIMENTS

Our length-matching tool is developed using C++ programming language. All experiments were performed with an AMD Ryzen 7840H 3.80 GHz CPU and 16GB memory.

### A. Overall Length-Matching Performance

The benchmark in this subsection is derived from the sample design provided by Allegro PCB Designer [22], in which we removed the tuning of preset matching groups and arranged 5 cases to evaluate the overall performance of length matching. Since we knew no published works concerned with the same objective and constraints of this problem, we compared our approach with the SOTA Auto-interactive Delay Tune (AiDT) function of Allegro PCB Designer, which is also applied in real-world industrial designs against the problem in this paper. The comparison is measured by the matching error of each

![Figure 14: Displays of our length-matching results. (a) Our experimental result. (b) Any direction functionality.](4495fbec19aac6861f1a0b35c4dc38bc_img.jpg)

Figure 14 consists of two sub-figures. Sub-figure (a) shows a complex PCB layout with multiple traces and pads, illustrating the experimental result of length matching. Sub-figure (b) shows a similar PCB layout, but with a different matching result, demonstrating the 'Any direction functionality' of the algorithm.

Figure 14: Displays of our length-matching results. (a) Our experimental result. (b) Any direction functionality.

Fig. 14. Displays of our length-matching results.

matching group and the runtime of the algorithm, the metrics of matching error are detailed as follows:

$$\begin{aligned}
\text{Max. error} &= \max_{i \in [1, L]} \frac{l_{target} - l_i}{l_{target}} \\
\text{Ave. error} &= \frac{\sum_i^L (l_{target} - l_i)}{n \times l_{target}}
\end{aligned}
\quad (19)$$

where  $l_i$  is the length of the  $i$ th trace after length matching and  $L$  is the number of traces in the case.

Table I gives case statistics and experimental results. As shown in the table, our approach addresses more precise length matching in the first 4 cases while compromising on runtime, which resulted from our DP-based extension having more flexible space utilization in spacing-dense environments and inevitable time complexity. Nevertheless, the presented runtime is still reasonable for human tolerance, and on the

![Figure 15: Displays of extension performance with and without DP. The figure consists of six sub-figures (a-f) showing different routing cases. (a) Case 1 with DP: A complex routing path with multiple vias and obstacles. (b) Case 5 with DP: A routing path with a single via and obstacles. (c) Case 6 with DP: A routing path with a single via and obstacles. (d) Case 1 without DP: A complex routing path with multiple vias and obstacles, showing a different routing strategy. (e) Case 5 without DP: A routing path with a single via and obstacles, showing a different routing strategy. (f) Case 6 without DP: A routing path with a single via and obstacles, showing a different routing strategy.](625e10f48104ba2b06b2220a9b224712_img.jpg)

Figure 15: Displays of extension performance with and without DP. The figure consists of six sub-figures (a-f) showing different routing cases. (a) Case 1 with DP: A complex routing path with multiple vias and obstacles. (b) Case 5 with DP: A routing path with a single via and obstacles. (c) Case 6 with DP: A routing path with a single via and obstacles. (d) Case 1 without DP: A complex routing path with multiple vias and obstacles, showing a different routing strategy. (e) Case 5 without DP: A routing path with a single via and obstacles, showing a different routing strategy. (f) Case 6 without DP: A routing path with a single via and obstacles, showing a different routing strategy.

Fig. 15. Displays of extension performance with and without DP.

TABLE II  
EXTENSION PERFORMANCE WITH AND WITHOUT DP

| case | $\frac{d_{gap}}{w_{trace}}$ | $\frac{l_{original}}{d_{gap}}$ | extension upper bound (%) |            |
|------|-----------------------------|--------------------------------|---------------------------|------------|
|      |                             |                                | with DP                   | without DP |
| 1    | 2.5                         | 24.89                          | 879.30                    | 845.80     |
| 2    | 3.0                         | 21.33                          | 718.79                    | 742.16     |
| 3    | 3.5                         | 18.67                          | 581.42                    | 345.62     |
| 4    | 4.0                         | 16.59                          | 481.14                    | 229.79     |
| 5    | 4.5                         | 14.93                          | 428.33                    | 177.92     |
| 6    | 5.0                         | 13.57                          | 327.41                    | 80.20      |

contrary, manually refining the delay tuning is usually pretty time-consuming. In case 5, our approach shows a similar length-matching precision to AiDT when spacing is sparse but wins an advantage in runtime. As the AiDT algorithm of Allegro is not public, we can only infer that it owes to the better efficiency of our MSDTW method.

Fig. 14a shows our experimental result, and Fig. 14b gives a dummy routing on a modified private commercial design that shows our functionality with any-direction traces.

### B. Ablation Experiments of DP

To examine the obstacle awareness of the proposed DP algorithm, we further conduct ablation experiments on a dummy design with narrow space between dense vias. The compared algorithm without DP is based on fixed routing tracks and constant pattern width. The statistics and results of experimental cases are given in Table II. In this table,  $w_{trace}$  denotes the width of the extended trace, and  $l_{original}$  denotes the trace length before length matching. The second column shows that we gradually increase  $d_{gap}$  to strengthen the restriction of DRC during the experiment. The third column indicates the ideal number of patterns that can be directly inserted perpendicular to the original according to the

ratio of  $l_{original}$  to  $d_{gap}$ . We evaluate the performance by measuring the upper bound of the extended length compared to the original length, which is calculated as:

$$\frac{l_{extended} - l_{original}}{l_{original}} \times 100\% \quad (20)$$

where  $l_{extended}$  is the trace length after length matching. Typically, we display three results of the cases in Fig. 15 to incorporate with the numerical metric.

It can be observed that the performance of the two algorithms is quite similar before the DRC restriction is strengthened enough. The algorithm with DP performs 3.96% better than the one without DP in case 1 while 3.15% worse in case 2. It is noteworthy that the result of case 2 is also reasonable because the former algorithm is essentially DP plus greedy, and the DP part is adopted for achieving the local optimum of segments but does not promise the global optimum of the whole trace.

In case 5, the algorithm without DP fails to utilize the space above the trace, and it also does not sufficiently utilize the space in the lower left area. It results from the algorithm cannot flexibly choose the patterns' feet or adjust the patterns' width. In this case, most of the fixed tracks leading to the upper area happen to be too close to the obstacles. The right part of the 135-degree segment in the middle seems can hold a pattern extended to the upper area, but the right foot of this pattern will actually violate  $d_{protect}$  away from the node of the original segments. Meanwhile, the empty space in the lower left part is just smaller than the square of  $2d_{gap} \times 2d_{gap}$ , so it is unable to further contain any pattern. While turning to the DP algorithm, these issues are all resolved properly. In the left and right areas, the algorithm wisely adjusts the patterns' width, routes around obstacles, and connects two patterns in opposite directions, which produces more space in the lower left part to contain a couple of continuous patterns. As for the middle part, the algorithm chooses the node of the original segments as a

![Figure 16: Example of the functionality of MSDTW. (a) shows an original differential pair (white) and its merged median trace (green). (b) shows a median trace (white) and its restored differential pair (green).](d336e7ffee4f537d0805ca840ec28582_img.jpg)

Figure 16: Example of the functionality of MSDTW. (a) shows an original differential pair (white) and its merged median trace (green). (b) shows a median trace (white) and its restored differential pair (green).

Fig. 16. Example of the functionality of MSDTW based on the case from the design in Fig. 9.

foot of the pattern, thereby avoiding the trouble of  $d_{protect}$ . All these improvements led by DP result in a significant advantage compared to another algorithm.

### C. MSDTW

Fig. 16a and Fig. 16b display the merged median trace and restored differential pair of a case from the design in Fig. 9, respectively.

## VII. CONCLUSION

In this paper, we present an automatic length-matching approach concerning any-direction traces in high-speed designs. Unlike previous works, we employ DP and computational geometry to meander the trace, which aims to preserve the specified original routing during length matching by maintaining direction and limiting overriding changes in topology. Meanwhile, it achieves more flexible obstacle-aware space utilization with reasonable runtime. Moreover, we proposed a method named MSDTW that converts differential pairs during length matching to tackle the issues against decoupling and multiple DRAs. The experimental illustration and the comparison with commercial tools demonstrate the effectiveness and functionality of our approach.

## ACKNOWLEDGMENTS

This work is supported by the National Natural Science Foundation of China (No. 12271098).

## REFERENCES

- [1] Y. Kubo, H. Miyashita, Y. Kajitani, and K. Tateishi, "Equidistance routing in high-speed VLSI layout design," in *Proceedings of the 14th ACM Great Lakes Symposium on VLSI*, ser. GLSVLSI '04. New York, NY, USA: Association for Computing Machinery, 2004, p. 220–223.
- [2] M. Ozdal and M. Wong, "Algorithmic study of single-layer bus routing for high-speed boards," *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, vol. 25, no. 3, pp. 490–503, 2006.
- [3] J.-T. Yan, "Single-layer obstacle-aware multiple-bus routing considering simultaneous escape length," *IEEE Transactions on Components, Packaging and Manufacturing Technology*, vol. 12, no. 6, pp. 902–915, 2022.
- [4] Y. Kohira, S. Suehiro, and A. Takahashi, "A fast longer path algorithm for routing grid with obstacles using biconnectivity based length upper bound," *IEICE transactions on fundamentals of electronics, communications and computer sciences*, vol. 92, no. 12, pp. 2971–2978, 2009.
- [5] Y. Kohira and A. Takahashi, "Cafe router: A fast connectivity aware multiple nets routing algorithm for routing grid with obstacles," *IEICE transactions on fundamentals of electronics, communications and computer sciences*, vol. 93, no. 12, pp. 2380–2388, 2010.
- [6] J.-T. Yan and Z.-W. Chen, "Obstacle-aware length-matching bus routing," in *Proceedings of the 2011 International Symposium on Physical Design*, ser. ISPD '11. New York, NY, USA: Association for Computing Machinery, 2011, p. 61–68.
- [7] M. Mustafa Ozdal and M. D. F. Wong, "A length-matching routing algorithm for high-performance printed circuit boards," *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, vol. 25, no. 12, pp. 2784–2794, 2006.
- [8] T. Yan and M. D. F. Wong, "Bsg-route: A length-constrained routing scheme for general planar topology," *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, vol. 28, no. 11, pp. 1679–1690, 2009.
- [9] Y.-J. Lee, H.-M. Chen, and C.-Y. Chin, "On simultaneous escape routing of length matching differential signalings," in *2013 IEEE Electrical Design of Advanced Packaging Systems Symposium (EDAPS)*, 2013, pp. 177–180.
- [10] R. Zhang and T. Watanabe, "A parallel routing method for fixed pins using virtual boundary," in *IEEE 2013 Tencon - Spring*, 2013, pp. 99–103.
- [11] Y. Nakatani and A. Takahashi, "A length matching routing algorithm for set-pair routing problem," *IEICE Transactions on Fundamentals of Electronics, Communications and Computer Sciences*, vol. 98, no. 12, pp. 2565–2571, 2015.
- [12] R. Zhang, T. Pan, L. Zhu, and T. Watanabe, "A length matching routing method for disordered pins in PCB design," in *The 20th Asia and South Pacific Design Automation Conference*, 2015, pp. 402–407.
- [13] T.-M. Tseng, B. Li, T.-Y. Ho, and U. Schlichtmann, "Ilp-based alleviation of dense meander segments with prioritized shifting and progressive fixing in PCB routing," *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, vol. 34, no. 6, pp. 1000–1013, 2015.
- [14] N. Kito, K. Takagi, and N. Takagi, "A fast wire-routing method and an automatic layout tool for RSFQ digital circuits considering wire-length matching," *IEEE Transactions on Applied Superconductivity*, vol. 28, no. 4, pp. 1–5, 2018.
- [15] S. Sato, K. Akagi, and A. Takahashi, "A fast length matching routing pattern generation method for set-pair routing problem using selective pin-pair connections," *IEICE Transactions on Fundamentals of Electronics, Communications and Computer Sciences*, vol. 103, no. 9, pp. 1037–1044, 2020.
- [16] C.-H. Hsu, S.-C. Hung, H. Chen, F.-K. Sun, and Y.-W. Chang, "A DAG-based algorithm for obstacle-aware topology-matching on-track bus routing," in *Proceedings of the 56th Annual Design Automation Conference 2019*, ser. DAC '19. New York, NY, USA: Association for Computing Machinery, 2019.
- [17] J. Chen, J. Liu, G. Chen, D. Zheng, and E. F. Y. Young, "MARCH: MAZE Routing under a Concurrent and Hierarchical scheme for buses," ser. DAC '19. New York, NY, USA: Association for Computing Machinery, 2019.
- [18] Y.-H. Cheng, T.-C. Yu, and S.-Y. Fang, "Obstacle-avoiding length-matching bus routing considering nonuniform track resources," *IEEE Transactions on Very Large Scale Integration (VLSI) Systems*, vol. 28, no. 8, pp. 1881–1892, 2020.
- [19] H. Sakoe and S. Chiba, "Dynamic programming algorithm optimization for spoken word recognition," *IEEE Transactions on Acoustics, Speech, and Signal Processing*, vol. 26, no. 1, pp. 43–49, 1978.
- [20] C. Myers, L. Rabiner, and A. Rosenberg, "Performance tradeoffs in dynamic time warping algorithms for isolated word recognition," *IEEE Transactions on Acoustics, Speech, and Signal Processing*, vol. 28, no. 6, pp. 623–635, 1980.
- [21] Y.-H. Chang, H.-T. Wen, and Y.-W. Chang, "Obstacle-aware group-based length-matching routing for pre-assignment area-I/O flip-chip designs," in *2019 IEEE/ACM International Conference on Computer-Aided Design (ICCAD)*, 2019, pp. 1–8.
- [22] Cadence, "Allegro PCB Designer." [Online]. Available: [https://www.cadence.com/en\\_US/home/tools/pcb-design-and-analysis/pcb-layout/allegro-pcb-designer.html](https://www.cadence.com/en_US/home/tools/pcb-design-and-analysis/pcb-layout/allegro-pcb-designer.html)

- [23] T.-Y. Tsai, R.-J. Lee, C.-Y. Chin, C.-Y. Kuan, H.-M. Chen, and Y. Kajitani, "On routing fixed escaped boundary pins for high speed boards," in *2011 Design, Automation & Test in Europe*, 2011, pp. 1–6.
- [24] S. Nakatake, K. Fujiyoshi, H. Murata, and Y. Kajitani, "Module packing based on the BSG-structure and IC layout applications," *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, vol. 17, no. 6, pp. 519–530, 1998.
- [25] D. Medhi and K. Ramasamy, "Chapter 4 - network flow models," in *Network Routing (Second Edition)*, second edition ed., ser. The Morgan Kaufmann Series in Networking, D. Medhi and K. Ramasamy, Eds. Boston: Morgan Kaufmann, 2018, pp. 114–157.
- [26] L. Luo, T. Yan, Q. Ma, M. D. Wong, and T. Shibuya, "B-escape: a simultaneous escape routing algorithm based on boundary routing," in *Proceedings of the 19th International Symposium on Physical Design*, ser. ISPD '10. New York, NY, USA: Association for Computing Machinery, 2010, pp. 19–25.
- [27] T.-H. Li, W.-C. Chen, X.-T. Cai, and T.-C. Chen, "Escape routing of differential pairs considering length matching," in *17th Asia and South Pacific Design Automation Conference*, 2012, pp. 139–144.