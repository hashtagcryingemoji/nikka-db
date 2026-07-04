package nikkadb.samples

import nikkadb.client.{NikkaClient, NikkaType, NikkaTypeWrapper}
import nikkadb.server.NikkaServer
import nikkadb.shared.Serializable
import nikkadb.shared.Serializable.given

object Main:
  def main(args: Array[String]): Unit = ()

  def basic(): Unit =
    val db = NikkaServer.withPort("0")
    val port = db.serverSocket.getLocalPort.toString

    new Thread(() => db.run()).start()

    Thread.sleep(100)

    val client = NikkaClient.withPort(port)

    client.setString("language:mascot:go", "gopher")
    client.setString("language:mascot:java", "duke")
    client.setString("language:framework:java", "spring")
    client.setString("language:framework:rust", "axum")

    println("all about java")
    for query <- client.getRegex("language:*:java") do
      println(s"$query - ${client.getString(query).getOrElse("undefined")}")

    println("take a look on some of the frameworks")
    for query <- client.getRegex("language:framework:*") do
      println(s"$query - ${client.getString(query).getOrElse("undefined")}")

    println("everything about everyone")
    for query <- client.getRegex("*:*:*") do
      println(s"$query - ${client.getString(query).getOrElse("undefined")}")

    client.setString("language:framework:typescript", "next.js")
    client.setString("language:framework:javascript", "react")

    println("know the difference!")
    for query <- client.getRegex("*:*:%%%%script") do
      println(s"$query - ${client.getString(query).getOrElse("undefined")}")

    println("so similar but so different")
    for query <- client.getRegex("*:framework:j*") do
      println(s"$query - ${client.getString(query).getOrElse("undefined")}")

  def transaction(): Unit =
    val db = NikkaServer.withPort("0")
    val port = db.serverSocket.getLocalPort.toString

    new Thread(() => db.run()).start()

    Thread.sleep(100)

    val client = NikkaClient.withPort(port)

    client.beginTransaction()
    client.setString("one", "1")
    client.eraseTransaction()
    client.setString("two", "2")
    client.sendTransaction()

    println(client.getString("one").getOrElse("undefined"))

  def deque(): Unit =
    val db = NikkaServer.withPort("0")
    val port = db.serverSocket.getLocalPort.toString

    new Thread(() => db.run()).start()

    Thread.sleep(100)

    val client = NikkaClient.withPort(port)

    client.createDeque("tasks", NikkaType.TypeString)
    client.pushFirst("tasks", NikkaTypeWrapper.NikkaString("eat"))
    client.pushLast("tasks", NikkaTypeWrapper.NikkaString("dota2"))
    client.pushLast("tasks", NikkaTypeWrapper.NikkaString("repeat"))
    println(client.popFirst[String]("tasks").get) // eat
