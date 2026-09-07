package dev.aic.host

import org.junit.Assert.*
import org.junit.Test

class AiProtocolTest {
    @Test fun reviewIsBoundedAndShowsChanges() {
        val review=AiProtocol.review("one\ntwo\n","one\nthree\n")
        assertTrue(review.contains("- two")); assertTrue(review.contains("+ three")); assertTrue(review.length<=12010)
    }
    @Test fun packageAndHashAreDeterministic() {
        val source="aic_version 0.1\napp \"x\" package \"dev.aic.example\" {}"
        assertEquals("dev.aic.example",AiProtocol.packageName(source))
        assertEquals(AiProtocol.hash(source),AiProtocol.hash(source))
    }
    @Test fun verticalCenterEditPreservesControlsAndAddsOnlyTwoSpacers() {
        val source="""aic_version 0.1
app "Counter" package "dev.aic.counter" {
  activity MainActivity {
    on_create {
      let root = android.linear_layout(orientation: vertical)
      let output = android.text_view(text: "Count")
      let increment = android.button(text: "Increment")
      android.add_view(parent: root, child: output)
      android.add_view(parent: root, child: increment)
      android.set_content_view(root)
    }
  }
}
"""
        val result=AiProtocol.knownEdit("Center all elements vertically",source)
        assertNotNull(result)
        val edited=result!!
        assertEquals(1,Regex("let top_spacer").findAll(edited).count())
        assertEquals(1,Regex("let bottom_spacer").findAll(edited).count())
        assertEquals(1,Regex("let increment =").findAll(edited).count())
        assertTrue(edited.indexOf("child: top_spacer") < edited.indexOf("child: output"))
        assertTrue(edited.indexOf("child: bottom_spacer") < edited.indexOf("set_content_view(root)"))
    }
    @Test fun applicationRenameChangesOnlyDisplayName() {
        val source="""aic_version 0.1
app "Counter" package "dev.aic.counter" {
  capability persistence.key_value
  preference saved_count: i32 = 0
  activity MainActivity { on_create { let root = android.linear_layout(orientation: vertical) android.set_content_view(root) } }
}
"""
        val edited=AiProtocol.knownEdit("Rename the application title to My Persistent Counter.",source)
        assertEquals(source.replace("app \"Counter\"","app \"My Persistent Counter\""),edited)
    }
    @Test fun localEndpointsRejectPublicCleartextAndCredentials() {
        assertEquals("http://127.0.0.1:11434/api/chat",ModelProviderFactory.localEndpoint("http://127.0.0.1:11434","/api/chat"))
        assertEquals("http://192.168.1.9:8080/v1/chat/completions",ModelProviderFactory.localEndpoint("http://192.168.1.9:8080/","/v1/chat/completions"))
        assertThrows(IllegalArgumentException::class.java) { ModelProviderFactory.localEndpoint("http://example.com:8080","/v1/chat/completions") }
        assertThrows(IllegalArgumentException::class.java) { ModelProviderFactory.localEndpoint("http://user:pass@127.0.0.1:8080","/v1/chat/completions") }
    }
}
