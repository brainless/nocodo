package com.nocodo.manager.ui.screens.servers

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import com.nocodo.manager.R
import com.nocodo.manager.domain.model.Server
import com.nocodo.manager.ui.components.ConnectionDialog

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ServersScreen(
    viewModel: ServersViewModel = hiltViewModel(),
    onMenuClick: () -> Unit
) {
    val servers by viewModel.servers.collectAsState()
    val sshKeyInfo by viewModel.sshKeyInfo.collectAsState()
    var showConnectionDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.servers_title)) }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showConnectionDialog = true }
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = stringResource(R.string.fab_connect_new_server)
                )
            }
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            if (servers.isEmpty()) {
                EmptyServersView()
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    items(servers) { server ->
                        ServerCard(
                            server = server,
                            onConnect = { viewModel.connectToServer(server) }
                        )
                    }
                }
            }
        }
    }

    if (showConnectionDialog) {
        ConnectionDialog(
            onDismiss = { showConnectionDialog = false },
            onConnect = { server ->
                viewModel.saveServer(server)
                viewModel.connectToServer(server)
                showConnectionDialog = false
            },
            sshKeyInfo = sshKeyInfo
        )
    }
}

@Composable
fun EmptyServersView() {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = stringResource(R.string.servers_empty),
            style = MaterialTheme.typography.titleMedium
        )
        Text(
            text = stringResource(R.string.servers_add_new),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
fun ServerCard(
    server: Server,
    onConnect: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Text(
                text = server.connectionString(),
                style = MaterialTheme.typography.titleMedium
            )

            Text(
                text = server.keyPath ?: stringResource(R.string.default_key),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp)
            )

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                horizontalArrangement = Arrangement.End
            ) {
                TextButton(onClick = onConnect) {
                    Text(stringResource(R.string.btn_connect))
                }
            }
        }
    }
}
