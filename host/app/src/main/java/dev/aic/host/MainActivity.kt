package dev.aic.host

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextWatcher
import android.text.method.ScrollingMovementMethod
import android.view.View
import android.view.MotionEvent
import android.view.WindowInsets
import android.widget.*
import java.util.UUID

class MainActivity : Activity() {
    private lateinit var store: ProjectStore
    private lateinit var controller: BuildController
    private lateinit var installer: AppInstaller
    private lateinit var ai: AiController
    private lateinit var source: EditText
    private lateinit var title: TextView
    private lateinit var output: TextView
    private lateinit var build: Button
    private lateinit var install: Button
    private lateinit var launch: Button
    private lateinit var level: Switch
    private lateinit var progress: ProgressBar
    private lateinit var aiPrompt: EditText
    private lateinit var aiRun: Button
    private lateinit var aiApply: Button
    private var project: StoredProject?=null
    private var rendering=false
    private var exportBytes: ByteArray?=null
    private val uiHandler=Handler(Looper.getMainLooper())
    private val saveLater=Runnable { guarded { save() } }
    private val state by lazy { getSharedPreferences("editor",MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store=ProjectStore(this); controller=BuildController.get(this); installer=AppInstaller(this); ai=AiController.get(this)
        val root=LinearLayout(this).apply { orientation=LinearLayout.VERTICAL; isFocusableInTouchMode=true; setPadding(dp(16),dp(12),dp(16),dp(12)); setBackgroundColor(0xfff5f6f8.toInt()) }
        root.setOnApplyWindowInsetsListener { v,insets ->
            val bars=insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.ime())
            v.setPadding(dp(16)+bars.left,dp(12)+bars.top,dp(16)+bars.right,dp(12)+bars.bottom); insets
        }
        title=TextView(this).apply { textSize=22f; setTypeface(null,Typeface.BOLD); text="AIC Host" }; root.addView(title)
        root.addView(TextView(this).apply { text="AIC source to Android app | API 35"; textSize=12f })
        fun row(vararg actions: Pair<String,()->Unit>): LinearLayout = LinearLayout(this).apply {
            actions.forEach { (label,action) -> addView(Button(this@MainActivity).apply {
                text=label; textSize=12f; isAllCaps=false; contentDescription=label; setOnClickListener { guarded(action) }
            },LinearLayout.LayoutParams(0,dp(48),1f)) }
        }
        root.addView(row("Projects" to { chooseProject() },"New" to { createProject() },"Import" to { importProject() },"Export" to { exportProject() }))
        val promptRow=LinearLayout(this).apply { orientation=LinearLayout.HORIZONTAL }
        aiPrompt=EditText(this).apply { hint="Describe an app or a change"; contentDescription="AI prompt"; maxLines=3; filters=arrayOf(android.text.InputFilter.LengthFilter(4000)) }
        promptRow.addView(aiPrompt,LinearLayout.LayoutParams(0,-2,1f))
        promptRow.addView(Button(this).apply { text="Clear"; isAllCaps=false; contentDescription="Clear AI prompt"; setOnClickListener { aiPrompt.setText("") } },LinearLayout.LayoutParams(dp(76),dp(48)))
        root.addView(promptRow,LinearLayout.LayoutParams(-1,-2))
        val aiButtons=row("AI create" to { runAi(AiOperation.CREATE) },"AI edit" to { runAi(AiOperation.PATCH) },"Review / apply" to { reviewAi() },"AI settings" to { aiSettings() })
        aiRun=aiButtons.getChildAt(0) as Button; aiApply=aiButtons.getChildAt(2) as Button; root.addView(aiButtons)
        level=Switch(this).apply { text="Optimize (O1)"; isChecked=true; setOnCheckedChangeListener { _,_ -> if(!rendering) { uiHandler.removeCallbacks(saveLater); uiHandler.postDelayed(saveLater,500); refresh() } } }
        root.addView(level)
        source=EditText(this).apply {
            id=R.id.source_editor; contentDescription="AIC source"; gravity=android.view.Gravity.TOP or android.view.Gravity.START
            typeface=Typeface.MONOSPACE; textSize=13f; setHorizontallyScrolling(true)
            inputType=android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE or android.text.InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
            filters=arrayOf(android.text.InputFilter.LengthFilter(ProjectCodec.MAX_SOURCE))
            setBackgroundColor(0xffffffff.toInt())
            addTextChangedListener(object: TextWatcher {
                override fun beforeTextChanged(s: CharSequence?,start: Int,count: Int,after: Int) {}
                override fun onTextChanged(s: CharSequence?,start: Int,before: Int,count: Int) { if(!rendering) { uiHandler.removeCallbacks(saveLater); uiHandler.postDelayed(saveLater,500); refresh() } }
                override fun afterTextChanged(s: Editable?) {}
            })
        }
        root.addView(source,LinearLayout.LayoutParams(-1,0,1f))
        val buttons=row("Build" to { save(); project?.let { controller.start(it) } },"Install" to { installBuilt() },"Launch" to { installer.launch(this) })
        build=buttons.getChildAt(0) as Button; install=buttons.getChildAt(1) as Button; launch=buttons.getChildAt(2) as Button
        root.addView(buttons)
        progress=ProgressBar(this,null,android.R.attr.progressBarStyleHorizontal).apply { isIndeterminate=true }; root.addView(progress,LinearLayout.LayoutParams(-1,dp(4)))
        output=object: TextView(this) {
            override fun dispatchTouchEvent(event: MotionEvent): Boolean {
                parent?.requestDisallowInterceptTouchEvent(event.actionMasked != MotionEvent.ACTION_UP && event.actionMasked != MotionEvent.ACTION_CANCEL)
                return super.dispatchTouchEvent(event)
            }
        }.apply {
            textSize=12f; typeface=Typeface.MONOSPACE; setTextIsSelectable(true); contentDescription="Build diagnostics"
            movementMethod=ScrollingMovementMethod.getInstance(); isVerticalScrollBarEnabled=true; isNestedScrollingEnabled=true
            setPadding(dp(4),dp(4),dp(4),dp(4)); setBackgroundColor(0xffffffff.toInt())
        }
        root.addView(output,LinearLayout.LayoutParams(-1,dp(130)))
        root.addView(row("Save" to { save(); toast("Project saved") },"AI history" to { showAiHistory() },"Cancel install" to { installer.cancel(); refresh() },"About" to { about() }))
        setContentView(ScrollView(this).apply { isFillViewport=true; addView(root,FrameLayout.LayoutParams(-1,-2)) })
        root.requestFocus()
        guarded { state.getString("project",null)?.let { show(store.load(it)) } }
        refresh()
    }
    private fun dp(value: Int)=(value*resources.displayMetrics.density).toInt()
    private fun about() {
        val notices=assets.open("toolchain/NOTICE.txt").bufferedReader().use { it.readText() }
        val versions=assets.open("toolchain/versions.txt").bufferedReader().use { it.readText() }
        val content=TextView(this).apply { setPadding(dp(16),dp(16),dp(16),dp(16)); text="AIC Host 0.8.0\nAI proposals use schema ${AiProtocol.SCHEMA_VERSION} and remain subject to local compiler validation.\n\n$versions\n$notices"; setTextIsSelectable(true) }
        AlertDialog.Builder(this).setTitle("About and licenses").setView(ScrollView(this).apply { addView(content) }).setPositiveButton("Close",null).show()
    }
    private fun data(): ProjectData?=project?.data?.copy(source=source.text.toString(),optLevel=if(level.isChecked) 1 else 0)
    private fun save() {
        uiHandler.removeCallbacks(saveLater)
        val current=project ?: return
        val data=data() ?: return
        if(data != current.data) project=store.save(current.copy(data=data))
    }
    private fun show(value: StoredProject) {
        rendering=true; project=value; source.setText(value.data.source); level.isChecked=value.data.optLevel==1; rendering=false
        state.edit().putString("project",value.id).apply(); refresh()
    }
    private fun chooseProject() {
        save()
        val all=store.list()
        if(all.isEmpty()) { toast("Create or import a project first"); return }
        AlertDialog.Builder(this).setTitle("Projects").setItems(all.map { it.data.name }.toTypedArray()) { _,index -> guarded { show(all[index]) } }.show()
    }
    private fun createProject() {
        save()
        val templates=arrayOf("Hello World","Counter","Calculator","Notes")
        val files=arrayOf("hello","counter","calculator","notes")
        AlertDialog.Builder(this).setTitle("New project").setItems(templates) { _,index ->
            val name=EditText(this).apply { setText(templates[index]); isSingleLine=true }
            val dialog=AlertDialog.Builder(this).setTitle("Project name").setView(name).setNegativeButton("Cancel",null).setPositiveButton("Create",null).create()
            dialog.setOnShowListener { dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener { guarded {
                val source=assets.open("templates/${files[index]}.aic").bufferedReader().use { it.readText() }
                    .replace(Regex("package \"[^\"]+\""),"package \"dev.aic.local.p${UUID.randomUUID().toString().replace("-","")}\"")
                show(store.create(ProjectData(name.text.toString().trim(),source))); dialog.dismiss()
            } } }; dialog.show()
        }.show()
    }
    @Suppress("DEPRECATION")
    private fun importProject() {
        save(); startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE),100)
    }
    @Suppress("DEPRECATION")
    private fun exportProject() {
        save(); val current=project ?: return
        exportBytes=ProjectCodec.encode(current.data)
        // Snapshot survives Activity recreation while the document picker is open.
        java.io.File(cacheDir,"pending-export.aicproject").writeBytes(exportBytes!!)
        startActivityForResult(Intent(Intent.ACTION_CREATE_DOCUMENT).setType("application/zip").addCategory(Intent.CATEGORY_OPENABLE)
            .putExtra(Intent.EXTRA_TITLE,current.data.name.replace(Regex("""[^\p{L}\p{N} _-]"""),"_")+".aicproject"),101)
    }
    @Deprecated("Platform document picker callback")
    override fun onActivityResult(requestCode: Int,resultCode: Int,intent: Intent?) {
        super.onActivityResult(requestCode,resultCode,intent)
        if(resultCode != RESULT_OK) return
        val uri=intent?.data ?: return
        guarded {
            if(requestCode == 100) {
                val bytes=contentResolver.openInputStream(uri)?.use { ProjectCodec.readLimited(it,ProjectCodec.MAX_ARCHIVE) } ?: error("Cannot open document")
                val data=if(bytes.size >= 4 && bytes[0]==0x50.toByte() && bytes[1]==0x4b.toByte()) ProjectCodec.decode(bytes)
                    else ProjectCodec.validate(ProjectData("Imported project",ProjectCodec.utf8(bytes)))
                show(store.create(data)); toast("Project imported")
            } else if(requestCode == 101) {
                val bytes=exportBytes ?: java.io.File(cacheDir,"pending-export.aicproject").readBytes()
                contentResolver.openOutputStream(uri,"wt")?.use { it.write(bytes) } ?: error("Cannot write document")
                exportBytes=null; java.io.File(cacheDir,"pending-export.aicproject").delete(); toast("Project exported")
            }
        }
    }
    private fun installBuilt() {
        if(InstallState.approval != null) { deliverApproval(); return }
        save(); val current=data() ?: return
        check(controller.projectId==project?.id && controller.digest==BuildController.hash(current)) { "Build the current source before installation" }
        val app=controller.result ?: error("Build a project first")
        if(!packageManager.canRequestPackageInstalls()) { installer.allowSource(this); return }
        installer.install(app); refresh()
    }
    private fun runAi(operation: AiOperation) {
        val prompt=aiPrompt.text.toString().trim(); val current=if(operation==AiOperation.PATCH) data()?.source else null
        if(operation==AiOperation.PATCH) check(project!=null) { "Open a project before requesting an edit" }
        ai.start(project?.id,operation,prompt,current,if(level.isChecked) 1 else 0)
        aiPrompt.setText(""); refresh()
    }
    private fun showAiHistory() {
        val entries=AiProvenance(this).recent()
        val content=TextView(this).apply {
            setPadding(dp(16),dp(12),dp(16),dp(12)); typeface=Typeface.MONOSPACE; setTextIsSelectable(true)
            setHorizontallyScrolling(true); isHorizontalScrollBarEnabled=true
            text=if(entries.isEmpty()) "No AI prompt history yet." else entries.joinToString("\n\n") { entry -> buildString {
                append(entry.time).append("  ").append(entry.provider).append('/').append(entry.model).append('\n')
                append(entry.operation.uppercase()).append(" attempt ").append(entry.attempt).append("\nPrompt: ").append(entry.prompt)
                if(entry.summary.isNotBlank()) append("\nAI action: ").append(entry.summary)
                append("\nResult: ").append(entry.result)
            } }
        }
        val horizontal=HorizontalScrollView(this).apply { isFillViewport=true; isHorizontalScrollBarEnabled=true
            addView(content,FrameLayout.LayoutParams(-2,-2)) }
        AlertDialog.Builder(this).setTitle("AI prompt history").setView(ScrollView(this).apply { addView(horizontal) })
            .setNegativeButton("Close",null).setNeutralButton("Clear history") { _,_ -> confirmClearAiHistory() }.show()
    }
    private fun confirmClearAiHistory() {
        AlertDialog.Builder(this).setTitle("Clear AI history?").setMessage("This removes the private prompt and response history from this host installation.")
            .setNegativeButton("Cancel",null).setPositiveButton("Clear") { _,_ -> guarded { AiProvenance(this).clear(); toast("AI history cleared") } }.show()
    }
    private fun reviewAi() {
        val result=ai.outcome ?: error("No validated AI proposal is ready")
        val proposal=result.proposal ?: error(result.diagnostic)
        val before=if(proposal.operation==AiOperation.PATCH) data()?.source.orEmpty() else ""
        if(proposal.operation==AiOperation.PATCH) check(AiProtocol.hash(before)==result.originalHash) { "Source changed after generation; request a fresh edit" }
        val content=TextView(this).apply { setPadding(dp(16),dp(12),dp(16),dp(12)); typeface=Typeface.MONOSPACE; setTextIsSelectable(true)
            setHorizontallyScrolling(true); isHorizontalScrollBarEnabled=true
            text="${proposal.summary}\nAreas: ${proposal.touchedAreas.joinToString()}\n\n${AiProtocol.review(before,proposal.source)}" }
        val horizontal=HorizontalScrollView(this).apply { isFillViewport=true; isHorizontalScrollBarEnabled=true
            addView(content,FrameLayout.LayoutParams(-2,-2)) }
        AlertDialog.Builder(this).setTitle("Review validated proposal").setView(ScrollView(this).apply { addView(horizontal) })
            .setNegativeButton("Cancel",null).setPositiveButton("Apply") { _,_ -> guarded {
                if(proposal.operation==AiOperation.CREATE) {
                    val name=Regex("""app\s+"([^"]+)"""").find(proposal.source)?.groupValues?.get(1) ?: "AI project"
                    show(store.create(ProjectData(name,proposal.source,optLevel=if(level.isChecked) 1 else 0)))
                } else {
                    val current=project ?: error("Project is no longer open")
                    val appName=Regex("""\bapp\s+"([^"]+)"""").find(proposal.source)?.groupValues?.get(1) ?: current.data.name
                    show(store.save(current.copy(data=current.data.copy(name=appName,source=proposal.source,optLevel=if(level.isChecked) 1 else 0))))
                }
                ai.clear(); toast("Validated proposal applied")
            } }.show()
    }
    private fun aiSettings() {
        val settings=getSharedPreferences("ai-settings",MODE_PRIVATE)
        val box=LinearLayout(this).apply { orientation=LinearLayout.VERTICAL; setPadding(dp(16),0,dp(16),0) }
        val kinds=ProviderKind.entries
        val provider=Spinner(this).apply { adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,kinds.map { it.label }) }
        val key=EditText(this).apply { inputType=android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD }
        val model=EditText(this).apply { hint="Model name" }
        val url=EditText(this).apply { hint="Server base URL, reachable from this phone"; inputType=android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_URI }
        val localHelp=TextView(this).apply { text="For a PC server, use its private LAN URL or run adb reverse for the displayed loopback port."; textSize=12f }
        box.addView(provider); box.addView(key); box.addView(model); box.addView(url); box.addView(localHelp)
        fun render(kind: ProviderKind) {
            key.visibility=if(kind.credentialLabel==null) View.GONE else View.VISIBLE
            key.hint=kind.credentialLabel?.plus(" (blank keeps saved value)")
            model.setText(settings.getString("model.${kind.id}",kind.defaultModel))
            url.visibility=if(kind.defaultUrl==null) View.GONE else View.VISIBLE
            localHelp.visibility=url.visibility
            url.setText(settings.getString("url.${kind.id}",kind.defaultUrl))
        }
        provider.onItemSelectedListener=object: android.widget.AdapterView.OnItemSelectedListener {
            override fun onNothingSelected(parent: android.widget.AdapterView<*>?) {}
            override fun onItemSelected(parent: android.widget.AdapterView<*>?,view: View?,position: Int,id: Long) { key.setText(""); render(kinds[position]) }
        }
        val selected=ProviderKind.from(settings.getString("provider",ProviderKind.OPENAI.id)); provider.setSelection(kinds.indexOf(selected)); render(selected)
        AlertDialog.Builder(this).setTitle("AI provider settings").setView(box).setNegativeButton("Cancel",null).setNeutralButton("Clear credential") { _,_ ->
            SecretStore(this).saveCredential(kinds[provider.selectedItemPosition].id,"")
        }.setPositiveButton("Save") { _,_ -> guarded {
            val kind=kinds[provider.selectedItemPosition]; val chosenModel=model.text.toString().trim().ifBlank { kind.defaultModel }
            if(key.text.isNotBlank()) SecretStore(this).saveCredential(kind.id,key.text.toString().trim())
            if(kind.credentialRequired) check(SecretStore(this).credential(kind.id).isNotBlank()) { "${kind.label} ${kind.credentialLabel} is required" }
            val chosenUrl=if(kind.defaultUrl!=null) url.text.toString().trim() else null
            if(chosenUrl!=null) ModelProviderFactory.localEndpoint(chosenUrl,if(kind==ProviderKind.OLLAMA) "/api/chat" else "/v1/chat/completions")
            settings.edit().putString("provider",kind.id).putString("model.${kind.id}",chosenModel).apply { if(chosenUrl!=null) putString("url.${kind.id}",chosenUrl) }.apply()
            toast("${kind.label} settings saved")
        } }.show()
    }
    private fun deliverApproval() {
        InstallState.approval?.let { approval -> InstallState.approval=null; guarded { startActivity(approval) } }
    }
    private fun refresh() {
        if(!::source.isInitialized || !::build.isInitialized) return
        title.text=project?.data?.name ?: "AIC Host"
        source.isEnabled=project != null; level.isEnabled=project != null
        build.isEnabled=project != null && !controller.busy && !ai.busy
        aiRun.isEnabled=!ai.busy && !controller.busy
        aiApply.isEnabled=!ai.busy && ai.outcome?.proposal!=null
        val valid=project?.id==controller.projectId && data()?.let { BuildController.hash(it)==controller.digest }==true
        val session=installer.prefs.getInt("session",-1)
        install.isEnabled=InstallState.approval != null || (valid && controller.result != null && !controller.busy && session<0)
        launch.isEnabled=installer.prefs.getBoolean("installed",false)
        progress.visibility=if(controller.busy || ai.busy) View.VISIBLE else View.INVISIBLE
        val recovery=if(session>=0 && InstallState.approval==null) "\nAn installation is pending. Complete Android confirmation, or cancel and retry if it was interrupted." else ""
        output.text=controller.log+"\n"+ai.log+"\n"+installer.prefs.getString("status","").orEmpty()+recovery
    }
    override fun onResume() {
        super.onResume(); controller.changed={ refresh() }; ai.changed={ refresh() }; InstallState.changed={ refresh(); deliverApproval() }; refresh(); deliverApproval()
    }
    override fun onPause() { guarded { save() }; controller.changed=null; ai.changed=null; InstallState.changed=null; super.onPause() }
    override fun onDestroy() { uiHandler.removeCallbacks(saveLater); super.onDestroy() }
    private fun guarded(action: ()->Unit) { try { action() } catch(e: Exception) { if(::output.isInitialized) output.text="AIC6200: ${e.message}"; toast(e.message ?: "Operation failed") } }
    private fun toast(message: String) { Toast.makeText(this,message,Toast.LENGTH_LONG).show() }
}
