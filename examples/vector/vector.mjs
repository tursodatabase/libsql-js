import { pipeline } from "@xenova/transformers";
import { createReadStream } from "fs";
import { parse } from "csv-parse";
import Database from "libsql";

// Create a embeddings generator.
const extractor = await pipeline(
  "feature-extraction",
  "Xenova/jina-embeddings-v2-small-en",
  { quantized: false },
);

// Open a database file.
const db = new Database("movies.db");

// Create a table for movies with an embedding as a column.
db.exec("CREATE TABLE movies (title TEXT, year INT, embedding VECTOR(512))");

// Create a vector index on the embedding column.
db.exec("CREATE INDEX movies_idx USING vector ON movies (embedding)");

// Prepare a SQL `INSERT` statement.
const stmt = db.prepare(
  "INSERT INTO movies (title, year, embedding) VALUES (?, ?, vector(?))",
);

// Process a CSV file of movies generating embeddings for plot synopsis.
createReadStream("wiki_movie_plots_deduped.csv")
  .pipe(parse({ columns: true }))
  .on("data", async (data) => {
    const title = data.Title;
    const year = data.Year;
    const plot = data.Plot;
    const output = await extractor([plot], { pooling: "mean" });
    const embedding = output[0].data;
    stmt.run([title, year, embedding]);
  });
