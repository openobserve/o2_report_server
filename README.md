# O2 Report Server

To setup reports `ZO_REPORT_USER_EMAIL`, `ZO_REPORT_USER_PASSWORD` must be specified.  
Following are the ENVs related to chrome and SMTP. * means required -

**ENVs**

<table class="table table-striped table-bordered">
<thead>
<tr>
<th>ENV</th>
<th>Description</th>
<th>Default</th>
</tr>
</thead>
<tbody>

<tr><td colspan="3"><strong>Authentication</strong></td></tr>
<tr>
<td><code>ZO_REPORT_USER_EMAIL</code>*</td>
<td>Admin user email for report server access</td>
<td></td>
</tr>
<tr>
<td><code>ZO_REPORT_USER_PASSWORD</code>*</td>
<td>Admin user password</td>
<td></td>
</tr>

<tr><td colspan="3"><strong>HTTP Server</strong></td></tr>
<tr>
<td><code>ZO_HTTP_PORT</code></td>
<td>Port for the HTTP server</td>
<td>5090</td>
</tr>
<tr>
<td><code>ZO_HTTP_ADDR</code></td>
<td>Bind address for HTTP server</td>
<td>127.0.0.1</td>
</tr>

<tr><td colspan="3"><strong>Chrome Settings</strong></td></tr>
<tr>
<td><code>ZO_CHROME_PATH</code></td>
<td>Custom Chrome executable path</td>
<td>Auto-detected</td>
</tr>
<tr>
<td><code>ZO_CHROME_CHECK_DEFAULT_PATH</code></td>
<td>If `false`, it does not look for chromium in default locations (e.g. `CHROME` env, usual chrome file path etc.), Default is `true`.</td>
<td></td>
</tr>
<tr>
<td><code>ZO_CHROME_DOWNLOAD_PATH</code></td>
<td>If chromium can not be found in default locations and also `ZO_CHROME_PATH` is not specified, it downloads the system specific chromium in the given path. Default is `./data/download` (gitignored). `chromium` is downloaded for the first time only, afterwords, `chromium` is fetched from the given path. If there is any error regarding download of `chromium`, delete the download folder as it might be in a bad state.</td>
<td></td>
</tr>
<tr>
<td><code>ZO_CHROME_NO_SANDBOX</code></td>
<td>Disable Chrome sandbox</td>
<td>false</td>
</tr>
<tr>
<td><code>ZO_CHROME_SLEEP_SECS</code></td>
<td>Timeout for dashboard loading</td>
<td>20</td>
</tr>
<tr>
<td><code>ZO_CHROME_WINDOW_WIDTH</code></td>
<td>Browser window width</td>
<td>1024</td>
</tr>
<tr>
<td><code>ZO_CHROME_WINDOW_HEIGHT</code></td>
<td>Browser window height</td>
<td>730</td>
</tr>
<tr>
<td><code>ZO_CHROME_ADDITIONAL_ARGS</code></td>
<td>Additional Chrome command-line arguments (comma-separated). Use this to pass custom flags to Chrome</td>
<td></td>
</tr>
<tr>
<td><code>ZO_CHROME_DISABLE_DEFAULT_ARGS</code></td>
<td>Disable Chrome's default arguments. Set to true to launch Chrome without any default command-line arguments</td>
<td>false</td>
</tr>

<tr><td colspan="3"><strong>PDF Generation Settings</strong></td></tr>
<tr>
<td><code>ZO_PDF_LANDSCAPE</code></td>
<td>Paper orientation. Set to true for landscape, false for portrait</td>
<td>true</td>
</tr>
<tr>
<td><code>ZO_PDF_DISPLAY_HEADER_FOOTER</code></td>
<td>Display header and footer in the PDF</td>
<td>false</td>
</tr>
<tr>
<td><code>ZO_PDF_PRINT_BACKGROUND</code></td>
<td>Print background graphics in the PDF</td>
<td>false</td>
</tr>
<tr>
<td><code>ZO_PDF_SCALE</code></td>
<td>Scale of the webpage rendering</td>
<td>1.0</td>
</tr>
<tr>
<td><code>ZO_PDF_PAPER_WIDTH</code></td>
<td>Paper width in inches (US Letter is 8.5)</td>
<td>8.5</td>
</tr>
<tr>
<td><code>ZO_PDF_PAPER_HEIGHT</code></td>
<td>Paper height in inches (US Letter is 11)</td>
<td>11.0</td>
</tr>
<tr>
<td><code>ZO_PDF_MARGIN_TOP</code></td>
<td>Top margin in inches (~1cm)</td>
<td>0.4</td>
</tr>
<tr>
<td><code>ZO_PDF_MARGIN_BOTTOM</code></td>
<td>Bottom margin in inches (~1cm)</td>
<td>0.4</td>
</tr>
<tr>
<td><code>ZO_PDF_MARGIN_LEFT</code></td>
<td>Left margin in inches (~1cm)</td>
<td>0.4</td>
</tr>
<tr>
<td><code>ZO_PDF_MARGIN_RIGHT</code></td>
<td>Right margin in inches (~1cm)</td>
<td>0.4</td>
</tr>
<tr>
<td><code>ZO_PDF_PREFER_CSS_PAGE_SIZE</code></td>
<td>Prefer page size as defined by CSS. If false, content will be scaled to fit paper size</td>
<td>false</td>
</tr>
<tr>
<td><code>ZO_PDF_GENERATE_TAGGED_PDF</code></td>
<td>Generate tagged (accessible) PDF for screen readers and assistive technologies</td>
<td>false</td>
</tr>
<tr>
<td><code>ZO_PDF_GENERATE_DOCUMENT_OUTLINE</code></td>
<td>Embed the document outline into the PDF for navigation</td>
<td>false</td>
</tr>

<tr><td colspan="3"><strong>SMTP Settings</strong></td></tr>
<tr>
<td><code>ZO_SMTP_HOST</code>*</td>
<td>SMTP server host</td>
<td>localhost</td>
</tr>
<tr>
<td><code>ZO_SMTP_PORT</code>*</td>
<td>SMTP server port</td>
<td>25</td>
</tr>
<tr>
<td><code>ZO_SMTP_USER_NAME</code>*</td>
<td>SMTP authentication username</td>
<td></td>
</tr>
<tr>
<td><code>ZO_SMTP_PASSWORD</code>*</td>
<td>SMTP authentication password</td>
<td></td>
</tr>
<tr>
<td><code>ZO_SMTP_REPLY_TO</code></td>
<td>The user email whom people can reply to. Not being used yet.</td>
<td></td>
</tr>
<tr>
<td><code>ZO_SMTP_FROM_EMAIL</code>*</td>
<td>The user email that is going to send the email.</td>
<td></td>
</tr>
<tr>
<td><code>ZO_SMTP_ENCRYPTION</code></td>
<td>SMTP encryption method. Possible values - `starttls` and `ssltls` or can be ignored in case of `localhost:25`</td>
<td></td>
</tr>

<tr><td colspan="3"><strong>General Settings</strong></td></tr>
<tr>
<td><code>ZO_LOCAL_MODE</code></td>
<td>Enable local storage mode</td>
<td>true</td>
</tr>

</tbody>
</table>

**Example ENV setup**

```
ZO_REPORT_USER_EMAIL = "root@example.com"
ZO_REPORT_USER_PASSWORD = "Complexpass#123"

# HTTP
ZO_HTTP_PORT = 5090
ZO_HTTP_ADDR = "127.0.0.1"
ZO_HTTP_IPV6_ENABLED = false

# SMTP
ZO_SMTP_HOST = "smtp.gmail.com"
ZO_SMTP_PORT = 465 # Or 587
ZO_SMTP_USER_NAME = "mail@mail.com"
ZO_SMTP_PASSWORD = "somepassword"
ZO_SMTP_FROM_EMAIL = "mail@mail.com"
ZO_SMTP_ENCRYPTION = "ssltls" # Or "starttls"

# Chrome

# Set the chromium path
# ZO_CHROME_PATH = ".\download\win64-1045629\chrome-win\chrome.exe"

# It will automatically download system specific chromium in the `./download` folder.
# ZO_CHROME_CHECK_DEFAULT_PATH = false

ZO_CHROME_WINDOW_WIDTH = 1440
ZO_CHROME_WINDOW_HEIGHT = 730

# Additional Chrome arguments (optional)
# ZO_CHROME_ADDITIONAL_ARGS = "--disable-gpu,--disable-dev-shm-usage"
# ZO_CHROME_DISABLE_DEFAULT_ARGS = false

# PDF Generation (Optional - defaults shown)
ZO_PDF_LANDSCAPE = true
ZO_PDF_SCALE = 1.0
ZO_PDF_PAPER_WIDTH = 8.5
ZO_PDF_PAPER_HEIGHT = 11.0
ZO_PDF_MARGIN_TOP = 0.4
ZO_PDF_MARGIN_BOTTOM = 0.4
ZO_PDF_MARGIN_LEFT = 0.4
ZO_PDF_MARGIN_RIGHT = 0.4
# ZO_PDF_PRINT_BACKGROUND = false
# ZO_PDF_DISPLAY_HEADER_FOOTER = false
# ZO_PDF_PREFER_CSS_PAGE_SIZE = false
# ZO_PDF_GENERATE_TAGGED_PDF = false
# ZO_PDF_GENERATE_DOCUMENT_OUTLINE = false
```

On the OpenObserve part, you need to include the below ENVs -
```
ZO_WEB_URL = "http://localhost:5080"
ZO_REPORT_SERVER_URL = http://localhost:5090
# And if ZO_BASE_URI is present, then that also must be specified
# ZO_BASE_URI = "/abc"
```

**Note:** If you don't specify `ZO_CHROME_CHECK_DEFAULT_PATH` ENV, then before downloading chromium, it will look for chromium in default locations -

1.  Check the CHROME env
2.  Check usual chrome file names in user path
3.  (Windows) Registry
4.  (Windows & MacOS) Usual installations paths

So turning this ENV off forces the application to download the chromium at the specified path. Some caveats of the auto download feature -
- Does not work on linux arm platform.
- Only the chromium is downloaded, and it expects all the dependency shared libraries (e.g. libatk-bridge-2.0.so.0, libatk-1.0.so.0 etc.) required for chrome to run to be already present in the system.