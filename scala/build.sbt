ThisBuild / scalaVersion := "3.3.3"
ThisBuild / organization := "nikkadb"
ThisBuild / version      := "0.1.0"

lazy val shared = (project in file("modules/shared"))
  .settings(name := "nikkadb-shared")

lazy val server = (project in file("modules/server"))
  .dependsOn(shared)
  .settings(
    name := "nikkadb-server",
    libraryDependencies += "org.scalameta" %% "munit" % "1.0.0" % Test
  )

lazy val client = (project in file("modules/client"))
  .dependsOn(shared)
  .settings(name := "nikkadb-client")

lazy val samples = (project in file("modules/samples"))
  .dependsOn(server, client)
  .settings(name := "nikkadb-samples")

lazy val e2eTests = (project in file("modules/e2e-tests"))
  .dependsOn(server, client)
  .settings(
    name := "nikkadb-e2e-tests",
    libraryDependencies += "org.scalameta" %% "munit" % "1.0.0" % Test
  )

lazy val root = (project in file("."))
  .aggregate(shared, server, client, samples, e2eTests)
  .settings(name := "nikka-db-scala")
