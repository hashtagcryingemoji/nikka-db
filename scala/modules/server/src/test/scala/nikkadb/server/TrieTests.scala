package nikkadb.server

import munit.FunSuite
import nikkadb.server.utils.TrieNode

class TrieTests extends FunSuite:

  test("trie insert") {
    val t = TrieNode()
    t.insert("six")
    assert(t.find("six"))
    assert(!t.find("si"))
    assert(!t.find("seven"))
  }

  test("trie remove") {
    val t = TrieNode()
    t.insert("six")
    t.remove("six")
    assert(!t.find("six"))
  }

  test("regex") {
    val t = TrieNode()
    val regex = "g*pherism"
    t.insert("gadsfpherism")
    t.insert("gosdfpherism")
    t.insert("gopherism")
    t.remove("gopherism")
    val v = t.findRegex(regex)
    assertEquals(v, List("gadsfpherism", "gosdfpherism"))
  }
