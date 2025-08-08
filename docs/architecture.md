# Crowdsourced Meal Map - System Architecture

## System Design Diagram

```mermaid
graph TB
    subgraph "Frontend Layer"
        UI[Next.js Frontend<br/>React 19, TypeScript]
        PWA[Progressive Web App<br/>Offline Support]
    end

    subgraph "API Gateway"
        API[Express.js API Server<br/>RESTful Endpoints]
    end

    subgraph "Database Layer"
        DB[(Supabase PostgreSQL<br/>Primary Database)]
        RLS[Row Level Security<br/>Auth Policies]
    end

    subgraph "Authentication"
        AUTH[Supabase Auth<br/>JWT Tokens]
        SOCIAL[Social Login<br/>Google, GitHub, etc.]
    end

    subgraph "Data Management"
        MEALS[Meals Collection<br/>Restaurant Data]
        REVIEWS[User Reviews<br/>Ratings & Comments]
        LOCATIONS[Location Data<br/>Geographic Info]
        USERS[User Profiles<br/>Preferences]
    end

    subgraph "External Services"
        MAPS[Maps API<br/>Geographic Services]
        STORAGE[File Storage<br/>Images & Media]
    end

    subgraph "Future AI Integration"
        AI[AI Recommendations<br/>Meal Suggestions]
        ML[Machine Learning<br/>Preference Analysis]
    end

    subgraph "Development Infrastructure"
        MONO[Turbo Monorepo<br/>Shared Packages]
        TYPE[TypeScript<br/>Type Safety]
        TOOLS[Modern Tooling<br/>ESLint, Prettier]
    end

    %% Frontend connections
    UI --> API
    UI --> AUTH
    PWA --> UI

    %% API connections
    API --> DB
    API --> AUTH
    API --> MAPS
    API --> STORAGE

    %% Database connections
    DB --> RLS
    DB --> MEALS
    DB --> REVIEWS
    DB --> LOCATIONS
    DB --> USERS

    %% Auth connections
    AUTH --> SOCIAL
    AUTH --> RLS

    %% Future AI connections
    AI -.-> MEALS
    AI -.-> REVIEWS
    AI -.-> USERS
    ML -.-> AI

    %% Development infrastructure
    MONO --> UI
    MONO --> API
    TYPE --> MONO
    TOOLS --> MONO

    %% Styling
    classDef frontend fill:#e1f5fe,stroke:#0277bd,stroke-width:2px
    classDef backend fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef database fill:#e8f5e8,stroke:#2e7d32,stroke-width:2px
    classDef external fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef future fill:#fce4ec,stroke:#c2185b,stroke-width:2px,stroke-dasharray: 5 5
    classDef dev fill:#f1f8e9,stroke:#558b2f,stroke-width:2px

    class UI,PWA frontend
    class API,AUTH,SOCIAL backend
    class DB,RLS,MEALS,REVIEWS,LOCATIONS,USERS database
    class MAPS,STORAGE external
    class AI,ML future
    class MONO,TYPE,TOOLS dev
```

## Architecture Overview

### Frontend Layer
- **Next.js Application**: React 19 with TypeScript for type safety
- **Progressive Web App**: Offline capabilities and mobile-first design

### Backend Services
- **Express.js API Gateway**: RESTful API endpoints for data management
- **Supabase Backend**: Authentication, database, and real-time features

### Data Management
- **PostgreSQL Database**: Structured data storage with Supabase
- **Row Level Security**: Fine-grained access control
- **Real-time Updates**: Live data synchronization

### Key Features
- **Crowdsourced Meal Discovery**: Community-driven restaurant and meal data
- **Location-based Search**: Geographic filtering and mapping
- **User Reviews & Ratings**: Community feedback system
- **Social Authentication**: Multiple login providers

### Development Infrastructure
- **Turbo Monorepo**: Efficient code sharing and build optimization
- **TypeScript**: End-to-end type safety
- **Modern Tooling**: ESLint, Prettier, and development best practices

## Usage Instructions

### For Miro:
1. Copy the mermaid code above
2. In Miro, add a "Code Block" or use a Mermaid integration
3. Paste the code to generate the diagram
4. Alternatively, use online tools like mermaid.live to generate an image, then import to Miro

### For Documentation:
This file can be used as reference documentation for the project architecture and shared with team members or stakeholders.
