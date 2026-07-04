package nikkadb.tests

import munit.FunSuite
import nikkadb.client.{NikkaClient, NikkaType, NikkaTypeWrapper}
import nikkadb.server.NikkaServer
import nikkadb.shared.Serializable.given

class E2ETests extends FunSuite:

  private def spawn(): (String) =
    val db = NikkaServer.withPort("0")
    val port = db.serverSocket.getLocalPort.toString
    new Thread(() => db.run()).start()
    Thread.sleep(100)
    port

  test("element insertion") {
    val port = spawn()
    val db = NikkaClient.withPort(port)
    db.setString("value", "key")
    assertEquals(db.getString("value"), Some("key"))
    db.setString("key", "value")
    db.setInt("one", 1)
    assertEquals(db.getInt("one").get, 1.toByte)
  }

  test("backup") {
    val port = spawn()
    val db = NikkaClient.withPort(port)
    for _ <- 0 until 200 do db.setString("key", "value")

    db.createDeque("numbers", NikkaType.TypeInt)
    db.pushFirst("numbers", NikkaTypeWrapper.NikkaInt(1))
    db.pushLast("numbers", NikkaTypeWrapper.NikkaInt(2))

    Thread.sleep(1000)

    val s2 = NikkaServer.withPort("2220")
    new Thread(() => s2.run()).start()
    Thread.sleep(100)

    val db2 = NikkaClient.withPort("2220")
    assertEquals(db2.getString("key"), Some("value"))
    assertEquals(db2.popFirst[Byte]("numbers"), Some(1.toByte))
  }

  test("element delete") {
    val port = spawn()
    val db = NikkaClient.withPort(port)
    db.setString("value", "key")
    db.remove("value")
    assertEquals(db.getString("value"), None)
  }

  test("transaction") {
    val port = spawn()
    val client = NikkaClient.withPort(port)

    client.beginTransaction()
    client.setString("key1", "value")
    client.eraseTransaction()
    client.setString("key2", "value")
    client.sendTransaction()

    assertEquals(client.getString("key1"), None)
    assertEquals(client.getString("key2").get, "value")
  }

  test("regex") {
    val port = spawn()
    val client = NikkaClient.withPort(port)
    client.setString("alice:bob", "bob")
    client.setString("bob:alice", "alice")
    val query = client.getRegex("*:*").sorted
    val real = List("alice:bob", "bob:alice").sorted
    assertEquals(query, real)
  }

  test("clear") {
    val port = spawn()
    val client = NikkaClient.withPort(port)
    client.setString("one", "two")
    client.setInt("three", 3)
    client.clearDatabase()
    assertEquals(client.getString("one"), None)
    assertEquals(client.getInt("three"), None)
  }

  test("deque") {
    val port = spawn()
    val client = NikkaClient.withPort(port)

    client.createDeque("numbers", NikkaType.TypeInt)
    client.pushFirst("numbers", NikkaTypeWrapper.NikkaInt(1))
    client.pushLast("numbers", NikkaTypeWrapper.NikkaInt(2))
    assertEquals(client.popFirst[Byte]("numbers").getOrElse(0.toByte), 1.toByte)
    assertEquals(client.popLast[Byte]("numbers").getOrElse(0.toByte), 2.toByte)
    assertEquals(client.popLast[Byte]("numbers").getOrElse(0.toByte), 0.toByte)

    client.createDeque("strings", NikkaType.TypeString)
    client.pushFirst("strings", NikkaTypeWrapper.NikkaString("one"))
    client.pushLast("strings", NikkaTypeWrapper.NikkaString("two"))
    assertEquals(client.popFirst[String]("strings").getOrElse("0"), "one")
    assertEquals(client.popLast[String]("strings").getOrElse("0"), "two")
    assertEquals(client.popLast[String]("strings").getOrElse("0"), "0")
  }
