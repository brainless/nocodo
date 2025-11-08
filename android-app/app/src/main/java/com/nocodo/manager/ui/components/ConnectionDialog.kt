package com.nocodo.manager.ui.components

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.nocodo.manager.R
import com.nocodo.manager.domain.model.Server
import com.nocodo.manager.ssh.SshKeyManager

@Composable
fun ConnectionDialog(
    onDismiss: () -> Unit,
    onConnect: (Server) -> Unit,
    sshKeyInfo: SshKeyManager.SshKeyInfo?
) {
    var host by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("22") }
    var keyPath by remember { mutableStateOf("") }

    val context = LocalContext.current

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(text = stringResource(R.string.dialog_connect_title))
        },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedTextField(
                    value = host,
                    onValueChange = { host = it },
                    label = { Text(stringResource(R.string.field_ssh_host)) },
                    placeholder = { Text(stringResource(R.string.field_ssh_host_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )

                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it },
                    label = { Text(stringResource(R.string.field_username)) },
                    placeholder = { Text(stringResource(R.string.field_username_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )

                OutlinedTextField(
                    value = port,
                    onValueChange = { port = it },
                    label = { Text(stringResource(R.string.field_port)) },
                    placeholder = { Text(stringResource(R.string.field_port_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                )

                OutlinedTextField(
                    value = keyPath,
                    onValueChange = { keyPath = it },
                    label = { Text(stringResource(R.string.field_ssh_key_path)) },
                    placeholder = { Text(stringResource(R.string.field_ssh_key_path_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )

                if (sshKeyInfo != null) {
                    Spacer(modifier = Modifier.height(8.dp))

                    Text(
                        text = stringResource(R.string.your_ssh_key),
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary
                    )

                    OutlinedTextField(
                        value = sshKeyInfo.publicKey,
                        onValueChange = {},
                        modifier = Modifier.fillMaxWidth(),
                        readOnly = true,
                        maxLines = 3,
                        trailingIcon = {
                            IconButton(
                                onClick = {
                                    copyToClipboard(context, sshKeyInfo.publicKey)
                                    Toast.makeText(
                                        context,
                                        context.getString(R.string.copied_to_clipboard),
                                        Toast.LENGTH_SHORT
                                    ).show()
                                }
                            ) {
                                Icon(
                                    imageVector = Icons.Default.ContentCopy,
                                    contentDescription = stringResource(R.string.btn_copy)
                                )
                            }
                        }
                    )

                    Text(
                        text = stringResource(R.string.ssh_fingerprint, sshKeyInfo.fingerprint),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val portInt = port.toIntOrNull() ?: 22
                    val server = Server(
                        host = host,
                        user = username,
                        port = portInt,
                        keyPath = keyPath.ifBlank { null }
                    )
                    onConnect(server)
                },
                enabled = host.isNotBlank() && username.isNotBlank()
            ) {
                Text(stringResource(R.string.btn_connect))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.btn_cancel))
            }
        }
    )
}

private fun copyToClipboard(context: Context, text: String) {
    val clipboardManager = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText("SSH Public Key", text)
    clipboardManager.setPrimaryClip(clip)
}
