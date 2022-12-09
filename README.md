# LIMITS: Limit-IM does not have ITS LIMITS

[![Rust](https://github.com/Limit-IM/limit-server/actions/workflows/rust.yml/badge.svg)](https://github.com/Limit-IM/limit-server/actions/workflows/rust.yml)
![lines](https://tokei.ekzhang.com/b1/github/limit-im/limit-server)

**Technology Stack**

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Redis](https://img.shields.io/badge/redis-%23DD0031.svg?style=for-the-badge&logo=redis&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![JWT](https://img.shields.io/badge/JWT-black?style=for-the-badge&logo=JSON%20web%20tokens)
![Prometheus](https://img.shields.io/badge/Prometheus-E6522C?style=for-the-badge&logo=Prometheus&logoColor=white)
![Grafana](https://img.shields.io/badge/grafana-%23F46800.svg?style=for-the-badge&logo=grafana&logoColor=white)

**Supported OS**

![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)
![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)


`LIMITS` is yet another fully open source, interoperable, decentralised real-time communication protocol!

---
[`中文文档`](README.zh-cn.md)

## TL;DR: What is this?

It is a new IM protocol and with implementation with federal-based governance.

In simple words, you can run your own server and connect to other servers quite easy.

## Wait! Could I host it on my AWS EC2 T or Azure B1 series machine?

Memory, storage usage and the ease of deployment are the **Tier-0** concerns for this project,
so ideally it will be able to run on 1c1g VPS.

Also, this product is very friendly to cloud infrastructure, all databases and metrics take into account that users may deploy on cloud SaaS.

## How about k8s? Does it scale well on the cluster?

Scaling horizontally is such a big challenge, not to mention the difficulty we have to consider for standalone deployments.
Horizontal scaling is even scarier for Ops if you can't automate it, so we'll focus on clustering ideas when we make a standalone version.
At current stage, we are trying to decouple the components and then will try to develop them in a cluster-friendly way.
