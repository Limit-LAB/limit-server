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


## Progress of the project
Since we started developing this project we have completed a lot of work and the current phase is still in the technical and theoretical validation phase.

We first spent a month looking at existing IM protocols and we identified a number of issues such as centralisation, encryption, open source and private deployment etc.
Secondly we spent a month looking at federation governance and we found a lot of issues such as how to get federation servers to communicate with each other, how to keep federation servers consistent with each other, how to synchronise data between federation servers etc.

We have recently been working on the actual code and theoretical exercises and we hope to have the first version ready for development between March and April.


## Why are we doing this project?
There are so many options for IM protocols nowadays, but they all have one problem in common: they are all centralised.
This means that they are all controlled by a company or organisation that can close your account at any time, or modify your messages without your knowledge.
This is a very serious problem as it can lead to a breach of your privacy, or theft of your account.

Although many IMs claim to implement end-to-end encryption, some of these encryptions are really unknown to us or even some IM implementations do not turn on encryption by default, which leads to the possibility of your messages being eavesdropped on by third parties.
Others are end-to-end encrypted but their server or client code is not open-sourced, so you don't know what it actually does and you can't verify its security yourself.

Another disadvantage of not being open source is that it can't be deployed privately. For example, IM projects with a high level of privacy for internal use are usually not that user-friendly, and it's difficult for users to get started with a new IM framework, and it's not easy for companies to maintain a framework.

So the problem we faced was that there was no IM protocol that combined decentralisation, encryption, open source and private deployment.

## What is our solution?
We focus on high availability, high security, high scalability, high customisability and high ease of deployment.
Our solution is a new IM protocol that is federation-based, which means it is made up of multiple servers, each of which is a separate entity that communicates with each other via a federation protocol.

We also use relatively new technologies such as distributed databases, distributed caching, and CRDT state synchronisation.

We have created a federated messaging protocol that runs on any server in the federation, which means you can deploy your clients on any server without worrying about your messages being intercepted by other servers.

We use CRDT technology to synchronise state across the federation network, which means that a group can continue to operate with a minimum of one live server, providing a very high level of availability.

## Social Attributes
Today's mainstream open source IMs do not have particularly strong social attributes. In order to achieve profitability and open source attributes at the same time we can combine blockchain technology to allow users to develop their own communities and create their own wealth such as emojis, wallpapers, themes, bubbles, etc. Moreover, the blockchain is open source and the server owner can choose to deploy it so that the server also has some profitability. This would allow both the server owner and the user to earn some revenue while maintaining the open source nature.



## Wait! Can I host it on my AWS EC2 T or Azure B1 series machine?
Memory usage and storage usage and ease of deployment are the **Level 0** concerns for this project.
So it should be able to run on a 1C1G chick.

Also, this product is very cloud infrastructure friendly, with all the databases and metrics taken into account for possible user deployment on a cloud SaaS.

## Does it scale well on my k8s cluster?
Scaling horizontally is a very big challenge, not to mention the difficulty I have to consider for standalone deployments.
Horizontal scaling that can't be automated is even scarier for Ops, so when I do a standalone deployment version, do my best to make it scalable.
At this stage I will try to decouple the components and then try to develop them in a cluster friendly way.
