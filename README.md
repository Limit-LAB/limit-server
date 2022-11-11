# LIMITS: Lemon's IM does not have ITS LIMITS

[![Rust](https://github.com/LemonHX/limit/actions/workflows/rust.yml/badge.svg)](https://github.com/LemonHX/limit/actions/workflows/rust.yml)
![lines](https://tokei.ekzhang.com/b1/github/limit-im/limit-server)

**Technology Stack**

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Redis](https://img.shields.io/badge/redis-%23DD0031.svg?style=for-the-badge&logo=redis&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![JWT](https://img.shields.io/badge/JWT-black?style=for-the-badge&logo=JSON%20web%20tokens)
![InfluxDB](https://img.shields.io/badge/InfluxDB-22ADF6?style=for-the-badge&logo=InfluxDB&logoColor=white)

**Supported OS**

![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)
![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)


`LIMITS` is yet another fully open source, interoperable, decentralised real-time communication protocol!

---
[`中文版在这里啦~~`](README.zh-cn.md)

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
