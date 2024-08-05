# O2 Report Server

To setup reports `ZO_REPORT_USER_EMAIL`, `ZO_REPORT_USER_PASSWORD` must be specified.  
Following are the ENVs related to chrome and SMTP. * means required -

**ENVs**

<table class="table table-striped table-bordered">

<thead>

<tr>

<th>ENV</th>

<th>Description</th>

</tr>

</thead>

<tbody>

<tr>

<td>`ZO_CHROME_PATH`</td>

<td>If chrome is enabled, custom `chrome` executable path can be specified. If not specified, it looks for chrome executable in default locations. If still not found, it automatically downloads a good known version of `chromium`.</td>

</tr>

<tr>

<td>`ZO_CHROME_CHECK_DEFAULT_PATH`</td>

<td>If `false`, it does not look for chromium in default locations (e.g. `CHROME` env, usual chrome file path etc.), Default is `true`.</td>

</tr>

<tr>

<td>`ZO_CHROME_DOWNLOAD_PATH`</td>

<td>If chromium can not be found in default locations and also `ZO_CHROME_PATH` is not specified, it downloads the system specific chromium in the given path. Default is `./data/download` (gitignored). `chromium` is downloaded for the first time only, afterwords, `chromium` is fetched from the given path. If there is any error regarding download of `chromium`, delete the download folder as it might be in a bad state.</td>

</tr>

<tr>

<td>`ZO_CHROME_NO_SANDBOX`</td>

<td>If `true`, it launches chromium in `no-sandbox` environment. Default is `false`</td>

</tr>

<tr>

<td>`ZO_CHROME_SLEEP_SECS`</td>

<td>Specify the number of timeout in seconds the headless chrome will wait until all the dashboard data is loaded. Default is `20` seconds.</td>

</tr>

<tr>

<td>`ZO_CHROME_WINDOW_WIDTH`</td>

<td>Specifies the width of the headless chromium browser. Default is `1370`</td>

</tr>

<tr>

<td>`ZO_CHROME_WINDOW_HEIGHT`</td>

<td>Specifies the height of the headless chromium browser. Default is `730`</td>

</tr>

<tr>

<td>`ZO_SMTP_HOST`*</td>

<td>The SMTP Host. Default - `localhost`</td>

</tr>

<tr>

<td>`ZO_SMTP_PORT`*</td>

<td>SMTP port. Default - `25`</td>

</tr>

<tr>

<td>`ZO_SMTP_USER_NAME`*</td>

<td>SMTP user name.</td>

</tr>

<tr>

<td>`ZO_SMTP_PASSWORD`*</td>

<td>SMTP user password.</td>

</tr>

<tr>

<td>`ZO_SMTP_REPLY_TO`</td>

<td>The user email whom people can reply to. Not being used yet.</td>

</tr>

<tr>

<td>`ZO_SMTP_FROM_EMAIL`*</td>

<td>The user email that is going to send the email.</td>

</tr>

<tr>

<td>`ZO_SMTP_ENCRYPTION`</td>

<td>SMTP encryption method. Possible values - `starttls` and `ssltls` or can be ignored in case of `localhost:25`</td>

</tr>

</tbody>

</table>

**Example ENV setup**

```
ZO_REPORT_USER_EMAIL = "root@example.com"
ZO_REPORT_USER_PASSWORD = "Complexpass#123"

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
```

On the OpenObserve part, you need to include the below ENVs -
```
ZO_WEB_URL = "http://localhost:5080"
ZO_REPORT_SERVER_URL = http://localhost:5090
# And if ZO_BASE_URI is present, then that also must be specified
# ZO_BASE_URI = "/abc"
```

**Note:** If you don’t specify `ZO_CHROME_CHECK_DEFAULT_PATH` ENV, then before downloading chromium, it will look for chromium in default locations -

1.  Check the CHROME env
2.  Check usual chrome file names in user path
3.  (Windows) Registry
4.  (Windows & MacOS) Usual installations paths

So turning this ENV off forces the application to download the chromium at the specified path. Some caveats of the auto download feature -
- Does not work on linux arm platform.
- Only the chromium is downloaded, and it expects all the dependency shared libraries (e.g. libatk-bridge-2.0.so.0, libatk-1.0.so.0 etc.) required for chrome to run to be already present in the system.