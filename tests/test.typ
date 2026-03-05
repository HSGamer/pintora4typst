#import "/typst-package/pintorita.typ": render, render-svg

= Rust Pintora Plugin Test

== Mindmap 1
#render(
  ```
  mindmap
  @param layoutDirection TB
  + UML Diagrams
  ++ Behavior Diagrams
  +++ Sequence Diagram
  +++ State Diagram
  +++ Activity Diagram
  ++ Structural Diagrams
  +++ Class Diagram
  +++ Component Diagram
  ```.text,
)

== Sequence Diagram 1
#render(
  ```
  sequenceDiagram
    title: Sequence Diagram Example
    autonumber
    User->>Pintora: Request diagram
    activate Pintora
    Pintora->>Pintora: Parse DSL
    Pintora->>User: Return SVG
    deactivate Pintora
  ```.text,
)

== Mindmap 2
#render(
  ```
  mindmap
  + Animal
  ++ Mammal
  +++ Dog
  +++ Cat
  ++ Bird
  +++ Eagle
  +++ Duck
  ```.text,
)

== Sequence Diagram 2
#render(
  ```
  sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello Bob, how are you?
    Bob-->>Alice: I am good thanks!
  ```.text,
)

== Sequence Diagram 3
#render(
  ```
  sequenceDiagram
    participant Server
    participant DB
    Server->>DB: Query User
    DB-->>Server: User Data
  ```.text,
)

== Mindmap 3
#render(
  ```
  mindmap
  + Technology
  ++ Web
  +++ HTML
  +++ CSS
  +++ JS
  ++ Systems
  +++ Rust
  +++ C
  ```.text,
)

== Sequence Diagram 4
#render(
  ```
  sequenceDiagram
    title: Simple Request
    Client->>Server: HTTP GET /
    Server-->>Client: 200 OK
  ```.text,
)

== Sequence Diagram 5
#render(
  ```
  sequenceDiagram
    A->>B: Step 1
    B->>C: Step 2
    C->>A: Step 3
  ```.text,
)

== Mindmap 4
#render(
  ```
  mindmap
  + Colors
  ++ Primary
  +++ Red
  +++ Blue
  +++ Yellow
  ++ Secondary
  +++ Green
  +++ Purple
  +++ Orange
  ```.text,
)

== Sequence Diagram 6
#render(
  ```
  sequenceDiagram
    title: Auth
    User->>App: Login
    App->>DB: Check
    DB-->>App: OK
    App-->>User: Dashboard
  ```.text,
)

== Mindmap 5
#render(
  ```
  mindmap
  + Planets
  ++ Inner
  +++ Mercury
  +++ Venus
  +++ Earth
  +++ Mars
  ++ Outer
  +++ Jupiter
  +++ Saturn
  ```.text,
)

== Sequence Diagram 7
#render(
  ```
  sequenceDiagram
    A->>A: Self
  ```.text,
)

== UTF-8 Encoding Test
#render(
  ```
  sequenceDiagram
    participant ユーザー
    participant サーバー
    ユーザー->>サーバー: 🚀 こんにちは! (Hello)
    サーバー-->>ユーザー: サーバーからの応答 (Response)
    @note left of ユーザー: 多言語サポート\n(Multi-language support)
  ```.text,
)

== A sample diagram
#render(
  ```
  componentDiagram
  @param layoutDirection TB
  @param edgeType polyline
  @param componentPadding 20
  @param componentBackground #ffffff
  @param componentBorderColor #000000
  @param groupBackground #ffffff
  @param groupBorderColor #000000
  @param edgeColor #000000
  @param relationLineColor #000000
  @param textColor #000000
  @param hideGroupType true

  node "Management Plane (CMS)" {
    [Web Server]
    interface "Universal Agent Protocol (UAP)"
    [Database]
    [Web Server] -- [Database]
    [Web Server] -- [Universal Agent Protocol (UAP)]
  }

  node "Execution Plane (Agents)" {
    node "Selenium" {
        [Selenium Agent] as agent1
        [Universal Agent Protocol (UAP)] -- [agent1]
        [Browser / Grid] as b1
        [agent1] -- [b1]
    }
    node "Playwright" {
        [Playwright Agent] as agent2
        [Universal Agent Protocol (UAP)] -- [agent2]
        [Browser] as b2
        [agent2] -- [b2]
    }
  }

  Client -- [Web Server] : HTTP
  ```.text,
)
