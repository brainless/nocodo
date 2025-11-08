package com.nocodo.manager.data.local

import androidx.room.Database
import androidx.room.RoomDatabase
import com.nocodo.manager.data.local.entities.ServerEntity

@Database(
    entities = [ServerEntity::class],
    version = 1,
    exportSchema = false
)
abstract class AppDatabase : RoomDatabase() {
    abstract fun serverDao(): ServerDao

    companion object {
        const val DATABASE_NAME = "nocodo_database"
    }
}
