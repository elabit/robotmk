mkdir /omd/sites/cmk/local/share/addons
chmod 755 !$



OMD[cmk]:~$ cat etc/apache/conf.d/check_mk.conf.conf 
```
Alias /cmk/check_mk/addons "/omd/sites/cmk/local/share/addons"
<Directory "/omd/sites/cmk/local/share/addons">
  Options +Indexes
  AllowOverride None
</Directory>

# This shares the check_mk agents delivered with the OMD
# version via HTTP
Alias /cmk/check_mk/agents /omd/sites/cmk/share/check_mk/agents
<Directory /omd/sites/cmk/share/check_mk/agents>
  Options +Indexes
  ...
  ...
```

# Custom Icons and actions

* ID: robotmk_last_log
  * Robot Framework: last HTML log
  * addons/robotmk/$HOSTNAME_URL_ENCODED$/$SERVICEDESC$/suite_last_log.html
* ID: robotmk_last_error_log
  * Robot Framework: last error HTML log
  * addons/robotmk/$HOSTNAME_URL_ENCODED$/$SERVICEDESC$/suite_last_error_log.html

## Action definition

user_icons_and_actions = {'robotmk_last_error_log': {'icon': 'robotmk80_dot',
                            'sort_index': 15,
                            'title': 'Robot Framework: last error HTML log',
                            'toplevel': True,
                            'url': ('addons/robotmk/$HOSTNAME_URL_ENCODED$/$SERVICEDESC$/suite_last_error_log.html',
                                    '_blank')},
 'robotmk_last_log': {'icon': 'robotmk80',
                      'sort_index': 14,
                      'title': 'Robot Framework: last HTML log',
                      'toplevel': True,
                      'url': ('addons/robotmk/$HOSTNAME_URL_ENCODED$/$SERVICEDESC$/suite_last_log.html',
                              '_blank')}}

## Action menu rule

globals().setdefault('service_icons_and_actions', [])

service_icons_and_actions = [
{'id': 'd692b9bd-b20a-4981-8a08-70bdf68d30a1', 'value': 'robotmk_last_error_log', 'condition': {'service_labels': {'robotmk/html_last_error_log': 'yes'}}, 'options': {'docu_url': 'cmkadmin'}},
{'id': '5369566d-dbf2-4bff-bd00-080b1a647f8a', 'value': 'robotmk_last_log', 'condition': {'service_labels': {'robotmk/html_last_log': 'yes'}}, 'options': {'docu_url': 'cmkadmin'}},
] + service_icons_and_actions