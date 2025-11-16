# Book Recommendation Graph Database

This extension adds Neo4j graph database support to visualize and query book relationships.

## Features

- **Book Nodes**: Each book is represented as a node with properties (title, author, categories, rating, etc.)
- **Semantic Relationships**: 
  - `SIMILAR_TO`: Books with high semantic similarity (based on embeddings)
  - `SAME_AUTHOR`: Books by the same author
  - `SAME_GENRE`: Books in the same genre/category
  - `SAME_THEME`: Books with similar themes
  - `READ_NEXT`: Suggested reading order
  - `PART_OF_SERIES`: Books in the same series

## Setup

### 1. Install Neo4j

Using Docker (recommended):
```bash
cd apps/api
docker-compose up -d
```

Or download from: https://neo4j.com/download/

### 2. Configure Environment Variables

Add to your `.env` file:
```bash
APP_NEO4J_URI=bolt://localhost:7687
APP_NEO4J_USER=neo4j
APP_NEO4J_PASSWORD=your_password_here
```

### 3. Build the Graph

First, ensure you have indexed books in Pinecone:
```bash
./src/scripts/index_books.sh ./data/books.csv
```

Then build the graph:
```bash
chmod +x src/scripts/build_graph.sh
./src/scripts/build_graph.sh
```

This will:
- Fetch all books from Pinecone
- Create book nodes in Neo4j
- Generate semantic relationships based on:
  - Author similarity
  - Genre/category overlap
  - Embedding-based semantic similarity

## API Endpoints

### Get Book Graph
```http
GET /api/graph/book?book_id={id}&depth={depth}
```
Returns the graph neighborhood for a specific book.

**Parameters:**
- `book_id` (required): The book ID
- `depth` (optional): Graph traversal depth (default: 2)

**Response:**
```json
{
  "nodes": [
    {
      "id": "book-123",
      "title": "The Hobbit",
      "author": "J.R.R. Tolkien",
      "categories": ["Fantasy", "Adventure"],
      "rating": 4.5,
      "year": 1937
    }
  ],
  "relationships": [
    {
      "from_id": "book-123",
      "to_id": "book-456",
      "relation_type": "SIMILAR_TO",
      "weight": 0.92
    }
  ]
}
```

### Get Similar Books
```http
GET /api/graph/similar?book_id={id}&limit={limit}
```
Returns books similar to the specified book.

### Search Books
```http
GET /api/graph/search?query={query}&limit={limit}
```
Search for books by title pattern.

### Get Graph Statistics
```http
GET /api/graph/stats
```
Returns statistics about the graph (total nodes and relationships).

## Visualizing the Graph

### Neo4j Browser

1. Open http://localhost:7474
2. Login with your credentials
3. Run Cypher queries:
```cypher
// Find books similar to "The Hobbit"
MATCH (b:Book {title: "The Hobbit"})-[r:SIMILAR_TO]->(similar:Book)
RETURN b, r, similar
LIMIT 25

// Find all books by an author and their connections
MATCH (b:Book {author: "J.R.R. Tolkien"})-[r]-(connected:Book)
RETURN b, r, connected
LIMIT 50

// Find the shortest path between two books
MATCH path = shortestPath(
  (b1:Book {title: "The Hobbit"})-[*]-(b2:Book {title: "Harry Potter and the Philosopher's Stone"})
)
RETURN path

// Get highly connected books (hubs)
MATCH (b:Book)
WITH b, size((b)-[]->()) as connections
WHERE connections > 10
RETURN b.title, b.author, connections
ORDER BY connections DESC
LIMIT 20
```

## Frontend Visualization

You can use libraries like:
- **D3.js**: For custom graph visualizations
- **Vis.js**: Network visualization
- **Cytoscape.js**: Graph theory library
- **React Force Graph**: React component for 3D/2D graphs

Example React component:
```typescript
import { ForceGraph2D } from 'react-force-graph';

function BookGraph({ bookId }: { bookId: string }) {
  const [graphData, setGraphData] = useState({ nodes: [], links: [] });

  useEffect(() => {
    fetch(`/api/graph/book?book_id=${bookId}&depth=2`)
      .then(res => res.json())
      .then(data => {
        const nodes = data.nodes.map(node => ({
          id: node.id,
          name: node.title,
          val: node.rating * 2
        }));
        
        const links = data.relationships.map(rel => ({
          source: rel.from_id,
          target: rel.to_id,
          value: rel.weight
        }));
        
        setGraphData({ nodes, links });
      });
  }, [bookId]);

  return (
    <ForceGraph2D
      graphData={graphData}
      nodeLabel="name"
      nodeAutoColorBy="group"
      linkDirectionalParticles={2}
    />
  );
}
```

## Performance Optimization

### Indexes
The system automatically creates these indexes:
- Book ID (unique constraint)
- Book title
- Book author
- Book rating

### Query Optimization

Use `LIMIT` in queries:
```cypher
MATCH (b:Book)-[r:SIMILAR_TO]->(similar:Book)
RETURN b, r, similar
LIMIT 100
```

Use indexes for filtering:
```cypher
MATCH (b:Book)
WHERE b.rating >= 4.0 AND b.author = "J.R.R. Tolkien"
RETURN b
```

### Batch Operations

The graph building script processes data in batches for better performance.

## Troubleshooting

### Connection Issues
```bash
# Check if Neo4j is running
docker ps | grep neo4j

# Check Neo4j logs
docker logs book-graph-neo4j
```

### Memory Issues
Increase heap size in docker-compose.yml:
```yaml
- NEO4J_dbms_memory_heap_max__size=4G
```

### Slow Queries
Add indexes for frequently queried properties:
```cypher
CREATE INDEX book_category_idx IF NOT EXISTS FOR (b:Book) ON (b.categories)
```

## Advanced Usage

### Custom Relationships

Add custom relationship types by extending `RelationType` in `services/neo4j.rs`:
```rust
pub enum RelationType {
    SimilarTo,
    SameAuthor,
    SameGenre,
    ReadNext,
    CustomType, // Add your custom type
}
```

### Graph Algorithms

Use Neo4j's Graph Data Science library for:
- PageRank (find influential books)
- Community detection (find book clusters)
- Similarity algorithms
- Path finding

Example:
```cypher
// Find communities of related books
CALL gds.louvain.stream('myGraph')
YIELD nodeId, communityId
RETURN gds.util.asNode(nodeId).title AS title, communityId
ORDER BY communityId
```

## Maintenance

### Backup
```bash
docker exec book-graph-neo4j neo4j-admin dump --database=neo4j --to=/backups/neo4j-backup.dump
```

### Restore
```bash
docker exec book-graph-neo4j neo4j-admin load --from=/backups/neo4j-backup.dump --database=neo4j --force
```

### Clear Graph
```bash
export CLEAR_GRAPH=true
./src/scripts/build_graph.sh
```

Or via Cypher:
```cypher
MATCH (n) DETACH DELETE n
```

## Resources

- [Neo4j Documentation](https://neo4j.com/docs/)
- [Cypher Query Language](https://neo4j.com/docs/cypher-manual/current/)
- [Neo4j Browser Guide](https://neo4j.com/docs/browser-manual/current/)
- [Graph Data Science Library](https://neo4j.com/docs/graph-data-science/current/)
