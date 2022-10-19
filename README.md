# LIMITS
> Lemon's IM does not have ITS LIMITS

[![Rust](https://github.com/LemonHX/limit/actions/workflows/rust.yml/badge.svg)](https://github.com/LemonHX/limit/actions/workflows/rust.yml)

![[JWT](http://jwt.io/)](http://jwt.io/img/badge-compatible.svg)

[`中文版在这里啦~~`](README.zh-cn.md)


`LIMITS` is yet another fully open source, interoperable, decentralised real-time communication protocol!

---
## TL;DR WTF is that
It is a new IM protocol and with implementation with federal-based governance.

## Wait! Could I host it on my AWS EC2 T or Azure B1 series machine?
Memory usage and storage usage and also the ease of deployment is the **Tier-0** concern for this project,
so ideally it runs on 1c1g.

Also, this product is very friendly to cloud infrastructure, and all databases and metrics take into account that users may deploy on cloud SaaS.

## ~~If I'm super rich~~ Does it scale well on my k8s cluster?
Scaling horizontally is a very big challenge, not to mention the difficulty I have to consider for standalone deployments.
Horizontal scaling is even scarier for Ops if you can't automate it, so I'll focus on clustering ideas when I make a standalone version.
At this stage I will try to decouple the components and then try to develop them in a cluster-friendly way.
